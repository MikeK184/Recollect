# Development tooling

Reviewed: 2026-10-18 against the current repository configuration.
Recollect's own product is Rust with React/Vite, PostgreSQL/pgvector and
Neo4j/GDS. The [technology baseline](../foundation/techstack.md) and accepted
[runtime ADR](../adr/0003-product-runtime.md) govern implementation.

## Repository-local tools

| Task | Tool and boundary |
| --- | --- |
| Rust definitions, references, types and diagnostics | [Rust Analyzer MCP](rust-semantic-navigation.md); allow indexing to finish before interpreting results |
| Repository context and call traces | [Graft MCP](rust-semantic-navigation.md); automatic refresh is structural, optional compiler edges need explicit LSP refresh |
| Structural navigation, including Cognee reference code | [CodeGraph CLI](codegraph.md); run status/sync before relying on changed relationships |
| Literal, configuration and documentation lookup | `rg` and targeted file reads |
| Current dependency guidance | Context7 and primary official interfaces; record failed or stale lookups |
| Authority/current-state routing | [recollect-doc-router](../../.agents/skills/recollect-doc-router/SKILL.md) |
| Documentation edits and lifecycle reconciliation | [recollect-doc-maintainer](../../.agents/skills/recollect-doc-maintainer/SKILL.md) |
| Browser/native product checks | Existing browser-control tools and the repository's Playwright/native fixtures |

`.codex/config.toml` owns Codex MCP and six role registrations. `opencode.json`
and `.opencode/agents/` own OpenCode's equivalent roles, Context7 and isolated
chrome-devtools. Both use root `AGENTS.md`, the shared repo skills and CodeGraph.
Personal models, permissions and UI settings stay outside project configuration.
Specialist agents run only when the user requests delegation.

### Claude Code

Start Claude Code from the repository (any nested directory works). Root
`CLAUDE.md` imports `AGENTS.md`, so instructions stay shared. `.mcp.json`
registers Context7, Rust Analyzer and Graft with the Codex commands; the
navigation launchers `cd` to `CLAUDE_PROJECT_DIR` when the host sets it.
`.claude/settings.json` denies `rust_analyzer_format`, `code_actions`, `rename`
and `set_workspace`, matching Codex's `enabled_tools`. `.claude/agents/` holds
explorer, architect, worker, qa and reviewer; explorer and reviewer also
disallow edit tools. `.claude/skills/` links to the shared skills.

Claude Code asks once to approve the project's `.mcp.json` servers; that
approval stays personal and is not committed. `CONTEXT7_API_KEY` is optional
and forwarded by name only. Run `./scripts/setup-rust-navigation.sh` first for
the navigation servers. Check registrations with `claude mcp list` or `/mcp`,
loaded instructions with `/memory`, and roles with `/agents`. A listed server
is not a connection: make a read-only call such as a Context7 library lookup.
When a Codex role changes, regenerate or compare the matching Claude role body.

A registration or empty index response is not proof of a working or absent
capability. The [Rust navigation proof](../mappings/rust-semantic-navigation-proof-2026-10-09.md)
records actual semantic/structural calls and sandbox limits. Current MCP calls
also resolved the scheduler's Rust definition during the October 18 docs audit.
Developer graphs do not prove product runtime behavior or create Brain topics.

## Checks for current implementation

Use the existing Cargo/npm manifests and lockfiles; dependency changes belong to
the owning slice. Typical local checks are:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace --lib
npm --prefix web run typecheck
./scripts/validate.sh
```

After API changes run `./scripts/generate-api.sh` and rebuild the web client.
Use the [local development runbook](local-development.md) for PostgreSQL/Neo4j
and focused native/browser tests. Native tests may be ignored by default and
need explicit disposable fixtures; a fast unit suite does not prove those paths.

For documentation/public-guide changes, also run:

```sh
npm --prefix site run build
npm --prefix site run check
```

The [Pages workflow](../../.github/workflows/pages.yml) builds/checks the site
and runs governance validation. It publishes static documentation, not the app.
Runtime/host tests and paid provider tests remain separate, explicitly scoped work.

## Scope and recovery

Keep setup, caches and launchers inside this repository. Use the specific tool
runbook when caches are missing; do not silently write home-level configuration,
install a global fallback or enable Graft's paid deep summarization. Preserve
ignored reference checkouts and unrelated running services.

Recollect's product plugins, Brain-managed MCP connections and optional private
runner are separate from developer MCPs. Customer/cloud credentials are not
required merely to edit documentation or inspect the source. Add another tool or
workflow skill only when a concrete task needs it.
