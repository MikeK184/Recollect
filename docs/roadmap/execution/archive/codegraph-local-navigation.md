# Local CodeGraph Navigation

Status: shipped
Owning epic: `docs/roadmap/epics/developer-tooling.md`
Work type: governance

## Summary

- Goal: A working repository-local CodeGraph CLI and a verified index including
  Cognee, usable by current and future Recollect development sessions.
- Non-goals: Product features, MCP registration, automatic watchers, global
  installation changes, publication, or Cognee source changes.
- Delivery shape: Local tooling, ignored derived index, documentation, and
  recorded command/source verification.

## Governing Sources

- [ADR 0002](../../../adr/0002-local-codegraph-navigation.md)
- [Navigation contract](../../../contracts/local-codegraph-navigation.md)
- [Repository governance contract](../../../contracts/repository-governance.md)
- [Owning epic](../../epics/developer-tooling.md)

## Scope

- In scope: Exact npm pin/lockfile, setup script, CLI launcher, root index
  configuration/exclusions, permissions-flow demonstration, runbook, evidence,
  and agent/developer documentation.
- Out of scope: Customer repositories, home configuration, product dependencies,
  application tests/services, automatic upgrades, commits, or deployment.
- Blockers: None; setup and bounded technical choices are authorized.

## Surface and Interface Changes

- Interfaces: `./scripts/setup-codegraph.sh` installs and initializes/syncs;
  `./.codex/tools/codegraph/codegraph` runs local navigation commands with
  Recollect as its working directory and defaults to `status` with no arguments.
- Storage: Ignored `.codegraph/` working-tree index and `.cache/` local caches;
  no application schema or persistent product data changes.
- Ownership: Recollect owns tooling/config/index. Cognee is an included
  read-only upstream Git checkout.

## Data and Authority

- Inputs: Local Recollect files plus Cognee's tracked/unignored source under
  the explicit nested-repo opt-in; npm packages pinned in a lockfile.
- Authority: Root configuration selects index scope; accepted ADR/contract
  govern setup. Source and focused tests govern conclusions about code.
- Blind spots: Unsupported syntax, dynamic edges, omitted files, and stale
  data may limit graph completeness; no deployed permission checks are exercised.

## States and Edge Cases

- Loading: Index commands show their progress and finish with an observable
  status; query commands remain separate from installation.
- Empty: A missing checkout or empty graph cannot pass acceptance; inspect
  checkout availability and inclusion settings before claiming success.
- Error: Failed npm/index/query commands return nonzero and retain diagnostic
  output; do not replace files or rebuild silently to hide a failure.
- Blocked: Missing runtime tools or registry access are reported with the
  failed command; unaffected documentation work can continue.
- No-access: No service credentials are required. Unreadable local source is
  reported as a coverage limitation.
- Duplicate or replay: Setup uses `npm ci` and syncs an existing database;
  it does not create a second Cognee-local index or overwrite upstream files.
- Stale data: Run `status` and explicit `sync`; configuration/scope changes
  may require a deliberate full `index` rebuild after review.
- Reconciliation divergence: Record missing callers or source/index mismatches;
  source inspection is authoritative and exact command results remain evidence.

## Integrations and Runtime Inputs

- Providers: Public npm registry for the locked CodeGraph package/platform
  bundle; local Git checkout for indexing. No MCP server is added.
- Environment: Node/npm for setup; launch with `CODEGRAPH_DIR`,
  `CODEGRAPH_INSTALL_DIR`, `CODEGRAPH_NO_DOWNLOAD`, `CODEGRAPH_NO_UPDATE_CHECK`,
  `CODEGRAPH_TELEMETRY`, `DO_NOT_TRACK`, and repository-local `TMPDIR` defaults.
  Initial setup sets `CODEGRAPH_NO_WATCH=0` and `CODEGRAPH_FORCE_WATCH=1` only
  for CLI initialization, avoiding the upstream optional Git-hook installer;
  CLI initialization itself starts no watcher.
- Secrets: No tokens required or written; exclude environment/key files and
  disable telemetry. Do not read credential contents during validation.
- Failure handling: Preserve nonzero exit codes; retry an interrupted install
  through the explicit setup command. Do not run global installers or uninstallers.

## Tests and Acceptance

- Automated: Shell syntax checks and existing offline `./scripts/validate.sh`.
  Avoid tests that only duplicate these simple wrapper commands.
- Manual: Install/version/status; initial index then incremental sync; exact
  permissions symbol/callers compared to source; Python/TypeScript file coverage
  and excluded-path inspection; launch from a nested directory; unchanged
  Cognee HEAD/status and all 3,593 tracked file contents/modes.
- Acceptance: Local CLI operates independently of PSA; a nonempty healthy root
  index includes Cognee Python and frontend TypeScript; actual permission
  navigation succeeds; docs include usable commands, limits, and refresh steps;
  governance passes without invoking Cognee code.

## Closeout

- Planned: Pinned local tooling, index, source-checked permissions query, docs,
  and validation.
- Shipped: Repo-local npm-locked CodeGraph 1.6.0, setup and navigation launcher,
  root index including Cognee, exclusions, refresh workflow, source-checked
  permissions navigation, agent guidance, runbook, and dated evidence.
- Not shipped: Product/runtime features and integrations excluded above.
- New blockers: None.
- Docs updated: ADR 0002 and navigation contract/indexes; developer-tooling epic
  and execution indexes; root agent guide; Codex README; stack/tooling guidance;
  CodeGraph runbook and evidence mapping with their indexes.
- Validation: Setup and repeat setup passed; 3,075 files, 42,928 nodes, and
  116,460 edges with complete status and no pending changes/references or file
  extraction errors. All 15 function/method caller entries matched Python AST
  call sites; four other entries were file/import references. Nested-directory
  navigation and impact worked. A temporary source addition/removal proved
  incremental sync and was cleaned up. Excluded paths were absent. Cognee's
  clean HEAD and all 3,593 tracked file contents/modes matched the baseline
  SHA256 recorded in the mapping; no Cognee-local index was created. Shell
  syntax and scripts/validate.sh passed, including governance lint and all 32
  tests. The governance linter also passed after archive/index reconciliation.
- Version: N/A: no Recollect release policy; CodeGraph dependency pinned to 1.6.0.
- Commit: uncommitted.
