# Local CodeGraph Navigation Contract

Status: accepted

## Source

[ADR 0002](../adr/0002-local-codegraph-navigation.md) and the user's
2026-09-13 request to set up the demonstrated CodeGraph CLI.

## Contract

- The launcher runs the repository's exact npm-locked CodeGraph 1.6.0 package;
  it never falls back to the PSA installation or a global executable.
- The default project is Recollect, including when invoked from a subdirectory.
  Root `codegraph.json` opts only `cognee/` into ignored nested-repo discovery.
- Install dependencies under `.codex/tools/codegraph/`; keep npm and temporary
  caches under `.cache/`, and graph state under `.codegraph/`. Ignore generated
  state in Git. No source, config, or Git changes occur inside `cognee/`.
- Disable usage telemetry, update checks, and runtime download fallback through
  the launcher's environment. Setup needs npm registry access; graph queries
  use the installed local runtime and require no service credentials.
- Exclude environment files, key files, dependency trees, caches, and build
  output from indexing. An index is local derived source data, never a secret
  scanner or authorization boundary.
- Setup installs dependencies, then initializes a missing index or syncs an
  existing index. Errors exit nonzero. No automatic full rebuild, upgrade,
  agent installer, global configuration edits, or background service is used.
- Agents use `status` and `sync` for freshness, then exact symbols/files and
  caller/impact navigation. Direct source and tests validate graph findings;
  literal/configuration/document discovery continues to use `rg`.
- Keep normal governance validation offline and independent of CodeGraph's
  installation and the Cognee checkout. Record installation/index/query proof
  separately from product runtime validation.

## Acceptance

Verify the local version, root and subdirectory invocation, successful initial
index and repeat sync, Python/TypeScript coverage, and a real Cognee permissions
query with callers checked against source. Inspect indexed paths for excluded
state. Compare Cognee's HEAD, worktree status, and tracked content/mode digest
before and after. Run `./scripts/validate.sh` and record exact limitations.

## Explicit Deferrals

CodeGraph MCP registration, automatic watching, a global command replacement,
upgrades beyond 1.6.0, product code, Cognee/Enola replacement, graph publication,
deployment, commits, and pushes are outside this slice.
