# 0019: Rust semantic navigation and Graft developer context

Status: accepted

## Decision

The user's 2026-10-09 instruction to use Rust Analyzer MCP and Graft authorizes
these complementary repository-local developer tools. Use the Rust-written
`zeenix/rust-analyzer-mcp` bridge and the official rust-analyzer binary for Rust
definitions, references, types, symbols and diagnostics. Use `trailhq/Graft`
(`@nanonets/graft`) for structural repository context and optional compiler-resolved
Rust call edges. This selects the Rust-capable Graft project, rather than the
similarly named `graftmap` whose Rust parser is planned.

Register both under `.codex/config.toml`. Keep launchers, dependency manifests,
installed binaries, indexes and setup/proof artifacts inside Recollect. Preserve
existing CodeGraph, Context7, Enola, agent roles and personal configuration.
Expose navigation/diagnostic tools; do not expose workspace rebinding. No global
Graft installer, host hook installer, deep generative summaries or cloud attachment
is selected. Structural setup needs no paid model call.

Graft upstream is a Node/TypeScript developer dependency. It is not linked into
Recollect's Rust product or made a mandatory deployed service. Product mapping
and canonical business logic remain Rust. The earlier Python GLiNER experiment
does not authorize a mandatory Python product runtime.

## Why

The existing CodeGraph index provides structural development navigation, but no
Rust Analyzer MCP or Graft server was registered. The `rust-analyzer` on PATH was
an uninstalled rustup shim. Rust semantic resolution complements structural
graph heuristics and gives actual compiler-backed type/definition evidence.

Graft 0.21.1 includes telemetry and background update/wiring maintenance. Its
normal initialization can modify home-level Codex configuration. Repository
launchers must prevent that behavior rather than copy the global quick start.
The [local contract](../contracts/rust-semantic-developer-navigation.md) specifies
the permitted setup/runtime boundary and actual MCP proof.

## Consequences

Developer tooling remains distinct from Brain memory, graph publication,
automatic entity/topic mapping, runtime authorization and deployed app performance.
Rust Analyzer may run Cargo build scripts/procedural macros for this trusted
workspace and create local analysis artifacts. Diagnostics are not a substitute
for focused tests. Record readiness, source coverage and unresolved call edges;
do not report a registered server as connected without a successful call.

## Supersession

Extends the developer tooling of [ADR 0002](0002-local-codegraph-navigation.md).
Its existing CodeGraph CLI remains; no product architecture decision is replaced.
The prior Context7-only Codex registration restriction in the governance contract
is superseded for these two explicitly authorized developer servers. The checker
retains exact secret-free configuration and tool allowlists for all three servers.
