# Rust Analyzer MCP and Graft navigation

## Purpose and Prerequisites

Repository-local developer navigation for Recollect. Product memory, entity/topic
mapping and graph publication remain Rust product work. The Rust Analyzer bridge
is Rust; Graft is an upstream Node developer tool.

Verified host: Apple Silicon macOS, Rust 1.94.0 with rust-src, Node 26.5.0,
npm 11.17.0, Git, curl, gzip, shasum and `/usr/bin/sandbox-exec`. No model key is
needed. Setup downloads public dependencies; runtime launchers deny networking
and writes outside this checkout. Other hosts currently fail explicitly.

## Procedure

From the repository root:

```bash
./scripts/setup-rust-navigation.sh
codex mcp get rust_analyzer
codex mcp get graft
```

Setup installs rust-analyzer release 2026-10-05, `rust-analyzer-mcp` 0.4.0 and
npm-locked `@nanonets/graft` 0.21.1. It uses an owned copy of the active toolchain's
standard-library sources and caches dependencies for offline analysis. A compiler
change selects a different source cache; rerun setup if it is missing. Setup does
not install missing rust-src into the home rustup toolchain.

Reopen the project/start a fresh Codex session to expose new server registrations.
The MCP commands find the current Git checkout from root or nested directories;
the tool launchers then select its root. Run them in Recollect rather than an
independent research checkout. Context7 and existing roles stay registered.
The Graft launcher explicitly selects `.cache/rust-navigation/graft-graph` for
both CLI and MCP calls; `graft check` must not look for a root `graft/` directory.

Use `rust_analyzer_definition`, `rust_analyzer_references` and
`rust_analyzer_hover` for exact Rust navigation. File positions are zero-based
lines and UTF-16 character offsets. The initial index may take several seconds;
an empty response during loading is not an absence result. Diagnostics can run
Cargo checks and may be incomplete when their upstream timeout is reached.

Use Graft's `graft_find_code`, `graft_file_api`, `graft_trace_calls`,
`graft_find_all`, `graft_repo_map` and `graft_check_freshness` for broader context.
For an explicit index refresh with compiler enrichment:

```bash
./.codex/tools/graft/graft build --lsp --no-follow-nested-repos --no-follow-submodules
./.codex/tools/graft/graft check
```

Ordinary queries automatically refresh structural data. That upstream refresh
does not rerun LSP enrichment; explicitly rebuild with `--lsp` after source edits
when those extra edges matter. Refresh failures can return the previous graph:
inspect freshness and confirm relationships in current source. CodeGraph remains
available under its [existing runbook](codegraph.md).

## Verification

The Rust proof client discovers the actual project registrations and launches
them from `crates/server`. It checks effective isolation, initializes both MCP
servers, resolves a cross-module Rust definition/signature/references and calls
real Graft file/caller/context/freshness tools:

```bash
CARGO_HOME="$PWD/.cache/rust-navigation/cargo" \
CARGO_TARGET_DIR="$PWD/.cache/rust-navigation/proof-target" \
cargo run --offline --manifest-path .codex/tools/rust-navigation-proof/Cargo.toml
./scripts/validate.sh
```

Expected output includes isolation, Rust MCP and Graft MCP PASS. Receipts stay
under `.cache/rust-navigation/`; they contain local code navigation, not keys.
The proof checks the shared sandbox policy by attempting a write-open of an
existing home config without writing bytes, a connection to an owned local TCP
listener, and a repository-local canary write. Home config bytes remain equal.

No generative/deep layer is selected. Graft MCP freshness also prints `NO GRAPH`
for its missing **deep manifest** and pending meaning tier; its separate
`graph check: OK` verifies the structural index. Do not interpret the deep-layer
message as a missing structural graph or run paid summarization to clear it.
Configuration discovery proves registration; the actual stdio calls prove local
tool operation. Exposure in the currently open Codex session is separate.

## Failure and Recovery

Missing dependency/toolchain cache: rerun setup; there is no global fallback.
Missing rust-src: report the prerequisite rather than modify home rustup defaults.
Loading/content-modified responses: allow indexing to finish, then retry a
bounded query. Persistent failures are visible in the proof stderr logs.

Graft telemetry is disabled, dotenv loading points to `/dev/null`, model/Brain
credentials are removed from its environment, and networking/home-write upkeep
is blocked by the sandbox. Do not run upstream `init`, global wiring, hooks,
`--deep` or cloud attachment for this workflow. Installed dependencies and
indexes are ignored; source manifests/lockfiles/launchers remain reviewable.
The [dated proof](../mappings/rust-semantic-navigation-proof-2026-10-09.md)
records coverage and limitations. Version: N/A; changes are uncommitted.
