# Rust Analyzer MCP and Graft local proof

Observed: 2026-10-09

The later graph investigation also uses actual Rust Analyzer definition/hover
calls from `graph/read.rs` to `retrieval/graph.rs`, alongside the source-candidate
cross-module calls. Both MCP servers and effective sandbox isolation pass again
in `.cache/rust-navigation/graph-deadline-navigation.log`. A CLI check initially
looked for the default root `graft/` directory despite the launcher's owned
`GRAFT_DIR`. The launcher now passes its explicit `--dir` to upstream; the proof
checks successful index discovery from `crates/server` as well as MCP freshness.
The structural caller output is approximate: ambiguous/generic Rust names still
require source confirmation or Rust Analyzer. These tools do not prove SQL or
runtime behavior.
Confidence: verified

## Sources and Method

User instruction: “use Rust. Use Rust Analyzer mcp+ graft.” No corresponding
project registrations existed before this slice. The PATH rust-analyzer was an
uninstalled rustup shim. Existing CodeGraph/Enola remain separate tools.

Primary sources inspected:

- [Rust-written MCP bridge](https://github.com/zeenix/rust-analyzer-mcp),
  crates.io 0.4.0 source and tool schemas, MIT.
- [Official rust-analyzer release](https://github.com/rust-lang/rust-analyzer/releases/tag/2026-10-05)
  and [configuration](https://rust-analyzer.github.io/book/configuration.html).
- [Graft upstream](https://github.com/trailhq/Graft), npm `@nanonets/graft`
  0.21.1 installed source/lockfile, MIT. This is the Rust-capable Graft, distinct
  from the similarly named `graftmap` project.
- [Official Codex stdio/project MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).
  Context7 resolved Rust Analyzer and Graft documentation; actual package source
  determined current telemetry/upkeep and LSP-refresh behavior.

Installed locally: rust-analyzer `0.3.3073-standalone (65ac641199 2026-10-04)`
from the 2026-10-05 release, bridge 0.4.0, Graft 0.21.1. Host rustc is 1.94.0,
Node 26.5.0 and npm 11.17.0. Setup is
[repository-owned](../../scripts/setup-rust-navigation.sh); proof is a
[Rust MCP client](../../.codex/tools/rust-navigation-proof/src/main.rs).

## Observations

- Setup completed and reran successfully. Bridge/analyzer live under
  `.codex/tools/rust-analyzer/installed`; Graft dependencies are npm-locked under
  `.codex/tools/graft`. Caches/indexes/build output stay under `.cache/`.
- Initial real Rust definition proof failed. A malformed boolean Cargo extra-env
  override was removed; standard-library offline metadata lacked `moto-rt` and
  other dependencies. Setup now fetches those dependencies using an owned,
  compiler-specific source copy, and readiness retries are bounded. Later live
  calls passed. Transient `content modified` LSP responses were observed during
  initial loading; they are not evidence of missing symbols.
- Root configuration discovery passed; the Rust proof subsequently loaded those
  exact registrations and launched both servers from `crates/server`.
  Rust initialize/list/definition/hover/references/symbols passed for
  `crate::db::preparation_tx` used in `auth.rs`, resolving `db.rs` and references
  back to the source file. Lines were discovered from source, with UTF-16 offsets.
- Graft initialize/list and all exercised file API, caller, ranked-context and
  structural-freshness requests passed. Six Graft tools are exposed; the Rust
  host allowlist excludes rebinding and mutation/refactoring tools.
- Explicit `--lsp` build initially found 573 compiler-resolved edges. Following
  dependency caching and governance updates, the final build found **4,424 nodes,
  12,128 edges and 447 indexed files**: 5,646 extracted, 3,479 inferred and
  **3,003 LSP-resolved edges**.
  This is a development graph, not a memory-quality score. Confidence classes
  remain separate; unresolved/inferred edges are not promoted by assumption.
- An edit to the governance fixture during an earlier build produced `STALE`
  and a nonzero setup result. A subsequent build with source stable completed
  with `graph check: OK`; the stale index was not accepted as synchronized.
- No indexed path was under references, environment files, dependency trees,
  build output or caches. No provider key or paid model request was needed.
- Effective shared sandbox proof passed: an existing home-config write-open and
  owned loopback connection were denied with permission errors; an owned repo
  canary write succeeded. The home config bytes stayed equal. Runtime launchers
  use that policy, local Cargo/TMP paths and offline Cargo. Graft dotenv loading
  is disabled, telemetry opts out, and model/Brain credentials are removed.
- Setup footprint at observation: roughly 2.5 GiB of ignored caches/compilation,
  368 MiB of Graft dependencies and 38 MiB of installed Rust tooling.

Local receipts: `.cache/rust-navigation/{rust,graft}-mcp-receipt.json`,
`isolation-receipt.txt`, setup/proof stderr logs. They are ignored local evidence.

## Translation and Limits

Both tools are installed, configured and proven through real local MCP calls.
The currently open host tool catalog has not been reloaded; a new session may
be required for direct Codex invocation. Registration alone is not that proof.

Graft includes telemetry and update/wiring upkeep despite older documentation
describing a simpler footprint. Runtime sandboxing blocks networking and writes
outside this checkout; global `init`/hook installation was not used. Setup needs
public registry/release downloads. Apple Silicon macOS is the only tested host.

Graft automatic refresh is structural-only and may fall back to an old graph on
refresh failure. Explicit `--lsp` is needed to restore compiler enrichment after
an edit. Deep summaries/concept nodes are absent intentionally. MCP freshness
prints a missing deep manifest separately from the successful structural check.

Rust semantic proof covers a real cross-module example, not exhaustive macro,
target or diagnostic coverage. Diagnoses may require Cargo and upstream timeout
handling. These tools do not fix product graph lock waits or implement automatic
entities, aliases and topics. The larger automatic-mapping objective stays open;
its product implementation remains Rust.

## Follow-up

The later semantic discovery repair additionally proves the actual queue call
to `representation` resolves to `crates/server/src/semantic.rs` with Rust Analyzer,
and Graft returns the semantic queue file API. All prior isolation, graph/mapping
navigation, MCP and nested-directory CLI controls pass in that same Rust client
run. Receipt: `.cache/rust-navigation/semantic-discovery-navigation.log`.

The bounded support-audit follow-up additionally resolves the scheduler's real
`support_discovery::schedule` call into the new Rust module. Graft returns its
`prepare`/`publish` file API. Actual stdio navigation, isolation and nested CLI
freshness pass in `.cache/rust-navigation/support-discovery-navigation.log`.
The product scheduler remains Rust; Graft is a developer dependency only.

Use the [runbook](../runbooks/rust-semantic-navigation.md) and new project session
for normal navigation. Refresh after edits and verify ambiguous relationships in
source. Governance lint, all 35 governance tests, memory-support baseline checking
and its 10 tests passed. The three new governance controls reject workspace
rebinding, global Rust MCP launchers and deep cloud Graft mode; local `init` and
`--deep` launcher attempts also exited 2 before running upstream commands. Shell
syntax, Rust proof formatting and `git diff --check` passed. No release, commit,
push or deployment is performed here.
