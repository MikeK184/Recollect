# Rust Analyzer MCP and Graft developer integration

Status: shipped
Owning epic: `docs/roadmap/epics/developer-tooling.md`
Work type: governance

## Summary

- Goal: Working repository-local Rust semantic MCP navigation and Graft context.
- Non-goals: Replace product graph/memory, change providers or personal defaults.
- Delivery shape: Local dependencies, launchers, project config, actual MCP proof.

## Governing Sources

[ADR 0019](../../../adr/0019-rust-semantic-developer-navigation.md),
[contract](../../../contracts/rust-semantic-developer-navigation.md),
[epic](../../epics/developer-tooling.md).

## Scope

- In scope: Local Rust bridge/analyzer and Graft installation, index and MCP.
- Out of scope: Global installers/hooks, deep model summaries, product runtime.
- Blockers: None for the verified local developer integration.

## Surface and Interface Changes

- Interfaces: Two project MCP entries, explicit setup and repo-owned launchers.
- Storage: Ignored installed binaries, caches and generated structural graph.
- Ownership: Developer tooling only; no Brain schema or graph mutation API.

## Data and Authority

- Inputs: Trusted Recollect working-tree Rust/web code and compiler metadata.
- Authority: Current source/tests; indexes and language-server results are evidence.
- Blind spots: Rust resolution readiness, incomplete structural edges, host reload.

## States and Edge Cases

- Loading: rust-analyzer indexing may take time; expose incomplete readiness.
- Empty: No symbol/hit is a bounded empty result, not absence proof.
- Error: Setup/runtime failures exit nonzero; no global fallback.
- Blocked: Unsupported runtime isolation is explicit.
- No-access: No new account/provider credentials are required.
- Duplicate or replay: Repeated installation/build refreshes owned artifacts.
- Stale data: Graft freshness and current Rust queries accompany navigation.
- Reconciliation divergence: Verify ambiguous relationships in actual source.

## Integrations and Runtime Inputs

- Providers: Official rust-analyzer releases, crates.io, npm and local stdio MCP.
- Environment: Local paths, disabled telemetry, explicit repository root.
- Secrets: No credential values enter config/docs or indexed environment files.
- Failure handling: Bounded startup/tool calls; terminate only owned proof processes.

## Tests and Acceptance

- Automated: Actual stdio initialize/list/call, semantic cross-module and Graft
  file/context controls, scope/exclusion/isolation checks and governance validation.
- Manual: Root/nested CLI configuration discovery and actual registered launches;
  the current host catalog reload remains a separately stated limit.
- Acceptance: Both tools installed/registered and real calls pass with boundaries
  preserved; unresolved compiler-edge/host limitations are recorded honestly.

## Closeout

- Planned: Local Rust semantic and Graft structural navigation with MCP proof.
- Shipped: Locally installed pinned Rust Analyzer/bridge and npm-locked Graft,
  two project MCP registrations, repository-bound offline launchers, owned
  compiler-specific source/dependency caches, real semantic/structural MCP calls
  from a nested directory and effective isolation proof.
- Not shipped: A current-session catalog reload, Linux verification, deep cloud
  summaries or product graph/entity/topic changes. The broader mapping objective
  and its product performance acceptance remain open under their own scope.
- New blockers: None.
- Docs updated: ADR/contract, repository configuration authority/checker and
  negative tests, owning slice/indexes, agent guide, setup/runbook and
  [dated proof](../../../mappings/rust-semantic-navigation-proof-2026-10-09.md).
- Validation: Repeated setup and synchronized LSP build passed; final real MCP
  initialize/list/calls resolved cross-module Rust definition/hover/references
  and Graft file/caller/context/freshness. Shared sandbox denied home writes and
  network while allowing repo writes. Excluded graph paths were absent; init/deep
  attempts were rejected. Governance lint, 35 governance tests, memory-support
  baseline/10 tests, Bash syntax, Rust formatting and diff whitespace passed.
- Version: N/A — developer tooling.
- Commit: uncommitted.
