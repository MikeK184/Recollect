# Repository-local Rust Analyzer MCP and Graft

Status: accepted

## Source

The user's 2026-10-09 request and
[ADR 0019](../adr/0019-rust-semantic-developer-navigation.md), with the existing
[repository boundary](repository-governance.md) and
[CodeGraph navigation](local-codegraph-navigation.md).

## Contract

Install the Rust Analyzer bridge and standalone rust-analyzer under the repo's
`.codex/tools/`, using ignored `.cache/` for downloads/builds. Install Graft with
a repository-owned npm manifest/lockfile and ignored dependency directory.
Launchers select this exact workspace regardless of caller working directory;
there is no unrelated/global tool fallback. Setup is explicit and repeatable.
Missing binaries/dependencies fail with an actionable setup command.

Project MCP registration preserves Context7 and roles. Allow Rust symbols,
definition, references, hover, completion and diagnostic queries. Workspace
rebinding is excluded. Position arguments retain upstream zero-based LSP units;
returned locations are checked against actual source. Graft exposes its six
structural navigation/freshness tools and uses the exact repo index.

Graft setup does not call `init`, add global hooks or attach a remote Brain.
Structural build and MCP refresh require no provider key or paid model call.
Disable telemetry and block its automatic network/home-write upkeep in runtime
launchers. Keep graph output ignored; exclude environment/key files, ignored
research repositories, dependency trees, caches and build output. Optional LSP
build uses the locally installed rust-analyzer and retains its unresolved edges
honestly. A structural graph is not canonical product memory.

Keep dependency/download/setup writes inside the repository. Do not modify home
Codex/rustup defaults or reference checkout files. Runtime environment flags are
not by themselves proof of network or filesystem isolation; verify the actual
launcher controls. macOS is the current verification host; unsupported isolation
hosts fail explicitly rather than silently enabling global upkeep.
Use an owned, compiler-specific standard-library source copy and local dependency
cache for Rust metadata. A compiler change must not silently reuse an older copy.
Both MCP launchers apply the shared repository-write-only, network-denied policy.

Record server registration, process initialization, tool discovery and successful
real semantic/structural queries separately. A new host session/reload may be
needed to expose the tools in Codex; independent stdio calls remain usable for
proof in the current session. Governance checks stay offline and do not start
these services.

## Acceptance

- Verify versions, root/subdirectory launchers and rerun setup behavior.
- Real MCP initialize/list/call succeeds for both servers on Recollect.
- Rust definition/reference/hover query resolves a real cross-module symbol;
  Graft returns real Rust file signatures and structural callers/context.
- Inspect excluded indexed paths, runtime side effects, paid-call absence and
  current local configuration discovery. Preserve home defaults and references.
- Run focused checks and `./scripts/validate.sh`; record limitations explicitly.

## Explicit Deferrals

Product graph replacement, automatic memory/topic mapping, deep cloud summaries,
provider policy changes, global host hooks, deployment, commits and pushes.
