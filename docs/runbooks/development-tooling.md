# Development tooling

Reviewed: 2026-10-19 against the repository configuration at `f1b6bb3`.
Recollect's product is Rust with React/Vite, PostgreSQL/pgvector and
Neo4j/GDS. The [technology baseline](../foundation/techstack.md) and accepted
[runtime ADR](../adr/0003-product-runtime.md) govern implementation. This
runbook is the single record of the developer tooling installed in the
repository, how to set it up and how to verify it. It replaces ADRs 0002 and
0019, their contracts and the tooling slices; see [History](#history).

## What is installed

| Task | Tool and boundary |
| --- | --- |
| Rust definitions, references, types, diagnostics | Rust Analyzer MCP (`zeenix/rust-analyzer-mcp` 0.4.0 over rust-analyzer 2026-10-05); allow indexing to finish before trusting results |
| Repository context and call traces | Graft MCP (`@nanonets/graft` 0.21.1, npm-locked); automatic refresh is structural only |
| Structural navigation, including Cognee reference code | [CodeGraph CLI](codegraph.md) 1.6.0; run `status`/`sync` before relying on changed relationships |
| Literal, configuration and documentation lookup | `rg` and targeted file reads |
| Current dependency guidance | Context7 and primary official interfaces; record failed or stale lookups |
| Authority and current-state routing | [recollect-doc-router](../../.agents/skills/recollect-doc-router/SKILL.md) |
| Documentation edits and lifecycle reconciliation | [recollect-doc-maintainer](../../.agents/skills/recollect-doc-maintainer/SKILL.md) |
| Browser and native product checks | Existing browser-control tools and the repository's Playwright/native fixtures |

Graft is an upstream Node developer dependency; it is not linked into the
product or a deployed service. Developer graphs do not prove product runtime
behavior and do not create Brain topics. The product's plugins, Brain-managed
MCP connections and private runner are separate from these developer MCPs.

## Host configuration

`AGENTS.md` is the shared instruction prose. `.codex/config.toml` owns Codex
MCP servers and six role registrations; `opencode.json` and `.opencode/agents/`
own OpenCode's roles, Context7 and an isolated chrome-devtools server;
Claude Code uses `CLAUDE.md` (imports `AGENTS.md`), `.mcp.json`,
`.claude/settings.json`, `.claude/agents/` and `.claude/skills/` (links to the
shared skills). Personal models, permissions and UI settings stay outside
project configuration. Specialist agents run only when delegation is requested.
`CONTEXT7_API_KEY` is optional and forwarded by name only.

Claude Code asks once to approve the project's `.mcp.json` servers; that
approval stays personal. `.claude/settings.json` denies `rust_analyzer_format`,
`code_actions`, `rename` and `set_workspace`, matching Codex's `enabled_tools`.
Workspace rebinding and mutating refactors are never exposed.

## Rust Analyzer MCP and Graft

Verified host: Apple Silicon macOS, Rust 1.94.0 with rust-src, Node 26.5.0,
npm 11.17.0, Git, curl, gzip, shasum and `/usr/bin/sandbox-exec`. No model key
is needed. Setup downloads public dependencies; runtime launchers deny
networking and writes outside this checkout. Other hosts fail explicitly.

```bash
./scripts/setup-rust-navigation.sh
codex mcp get rust_analyzer
codex mcp get graft
```

Setup installs the pinned bridge, rust-analyzer and Graft under `.codex/tools/`
and keeps downloads, caches and indexes in ignored `.cache/`. It uses an owned,
compiler-specific copy of the standard-library sources and caches dependencies
for offline analysis; a compiler change selects a different cache, so rerun
setup if it is missing. It does not modify home rustup or Codex defaults, run
Graft `init`, add hooks, attach a remote Brain or use `--deep` summaries.
Launchers disable telemetry, point dotenv loading at `/dev/null`, strip model and
Brain credentials, and run under the shared repository-write-only,
network-denied sandbox policy. Reopen the project or start a new session to
expose newly registered tools.

Rust positions are zero-based lines and UTF-16 offsets. An empty or
`content modified` response while loading is not an absence result. Diagnostics
can run Cargo checks and may be incomplete after the upstream timeout. Graft's
launcher selects `.cache/rust-navigation/graft-graph` for CLI and MCP calls.
Ordinary queries refresh structural data only; after source edits, refresh
compiler-resolved edges explicitly:

```bash
./.codex/tools/graft/graft build --lsp --no-follow-nested-repos --no-follow-submodules
./.codex/tools/graft/graft check
```

A failed refresh can return the previous graph: inspect freshness and confirm
ambiguous relationships in source. `graft_check_freshness` prints `NO GRAPH`
for the missing deep manifest; the separate `graph check: OK` verifies the
structural index. Do not run paid summarization to clear it.

## Verification

A registration or tool listing is not a connection. The Rust proof client loads
the actual project registrations, launches both servers from `crates/server`,
checks the sandbox (a home-config write-open and a loopback connection are
denied while a repository canary write succeeds and home bytes stay equal),
resolves a cross-module Rust definition, hover and references, and calls real
Graft file, caller, context and freshness tools:

```bash
CARGO_HOME="$PWD/.cache/rust-navigation/cargo" \
CARGO_TARGET_DIR="$PWD/.cache/rust-navigation/proof-target" \
cargo run --offline --manifest-path .codex/tools/rust-navigation-proof/Cargo.toml
./scripts/validate.sh
```

Expect isolation, Rust MCP and Graft MCP PASS. Receipts stay under
`.cache/rust-navigation/` and contain only local code navigation. The
2026-10-09 proof resolved `db::preparation_tx` across modules and built a
Graft graph of 4,424 nodes, 12,128 edges and 447 files (3,003 LSP-resolved
edges); later runs resolved graph, semantic-queue and support-discovery call
sites. Graft caller output is approximate for generic names. Exposure in a
currently open host session is separate from these stdio calls. For Claude Code
check `claude mcp list`, `/memory` and `/agents`, then make a read-only call
such as a Context7 library lookup.

## Checks for current implementation

```sh
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace --lib
npm --prefix web run typecheck
./scripts/validate.sh
```

After API changes run `./scripts/generate-api.sh` and rebuild the web client.
Use the [local development runbook](local-development.md) for PostgreSQL/Neo4j
and focused native/browser tests; a fast unit suite does not prove ignored
native paths. For documentation or public-guide changes also run
`npm --prefix site run build` and `npm --prefix site run check`. The
[Pages workflow](../../.github/workflows/pages.yml) builds the site and runs
governance validation; it publishes static documentation, not the app.

## Scope and recovery

Keep setup, caches and launchers inside this repository. When caches are
missing, rerun the setup script; there is no global fallback and no home-level
configuration. Missing rust-src is reported as a prerequisite. Preserve ignored
reference checkouts and unrelated running services. Add another tool or skill
only when a concrete task needs it.

## History

ADRs [0002](../adr/0002-local-codegraph-navigation.md) and
[0019](../adr/0019-rust-semantic-developer-navigation.md) are superseded by
this runbook. Their contracts, the developer-tooling and governance slice packs
(Rust navigation, Claude Code, OpenCode, CodeGraph, documentation skills,
bootstrap, documentation cleanup and the product docs site) and the Rust
navigation and public-site mappings were removed on 2026-10-19. They remain
readable at commit
[`f1b6bb3`](https://github.com/MikeK184/Recollect/tree/f1b6bb3040dfe65b7fac90359af782cec1f7df31/docs).
