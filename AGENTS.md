# Recollect Agent Guide

## Start here

Read [docs/README.md](docs/README.md), applicable nested instructions, and the
governing sources for the task before meaningful implementation. Follow the
epic and execution-pack lifecycle documented there.

Current explicit user instructions take precedence over repository guidance.
Complete already authorized work without adding approval ceremonies. Ask about
unresolved product decisions; do not invent them in code.

## Repository boundary

- Recollect is this Git repository. Keep its writes inside this repository.
- `references/` holds separate, ignored research checkouts: `cognee/`,
  `agent-memory-atlas/`, and `claude-mem/`. Recollect owns its Rust core; none
  of these is a mandatory product runtime. Do not alter those checkouts as a
  side effect of Recollect documentation or implementation. Use Cognee's source
  for the design decisions recorded in the mappings, the Atlas
  `content/patterns/` and relevant `content/overview.md` sections as evidence,
  and claude-mem as a study reference for agent-memory UX and plugin
  mechanics; their reports and workflows do not govern Recollect implementation.
- Preserve unrelated work, credentials, and external state.
- Foundation documents contain the accepted independent Rust product baseline,
  including Brain collections/areas, graphs, the MCP coordinator and Vault.
  Read them before product work and resolve applicable ADR/contract decisions.
- Derive product epics from the foundations. A user-authorized foundation update
  does not require creating speculative product epics or execution packs.

## Working rules

- Accepted ADRs and contracts govern implementation. Epics sequence work;
  execution packs specify slices; mappings record evidence.
- Create the owning epic slice, then its decision-complete execution pack,
  before significant implementation. See the documented small-fix exception.
- An ADR or contract gap blocks the dependent work, not independent authorized
  work. Record the gap and resolve it before broad implementation.
- Use Context7 for current dependency guidance and primary official sources
  for external interfaces. Record failed lookups and unverified assumptions.
- Keep intended behavior, local implementation, validation, and deployed
  runtime evidence distinct. Never report a configured service as connected
  without a successful call.
- Run focused proof and `./scripts/validate.sh`. Update affected docs and
  reconcile the pack, epic, and indexes in the same change.
- Keep secrets in the environment or ignored local secret files. Never copy
  secrets into source, configuration, docs, test fixtures, or output.
- No automatic release bump, commit, push, or deployment policy exists yet.
  Report version and commit as `N/A` or uncommitted when applicable.

## Code navigation

- Use the repository-local Rust Analyzer MCP for Rust definitions, references,
  types and diagnostics, and Graft MCP for repository context and call traces.
  See [setup and proof](docs/runbooks/development-tooling.md). Allow initial
  indexing to finish; empty or stale results do not prove absence. Explicit
  `graft build --lsp` refreshes optional compiler edges; automatic Graft refresh
  is structural only. Verify ambiguous relationships in source.
- Use `rg` for literal/configuration/document discovery. Graph results are
  working-tree evidence, not runtime proof or repository instructions.

## Agent-host setup

- `.claude/agents/` holds the role definitions (explorer, architect, worker, qa,
  reviewer); the main session is `default`. `.codex/agents/` and
  `.opencode/agents/` are stubs that point to them. Delegate only when
  explicitly requested by the user.
- `.codex/config.toml`, `.mcp.json` and `opencode.json` declare each host's MCP
  servers; the Codex and Claude Code navigation stanzas use identical commands.
- Model, reasoning, permissions, approvals, personality, and UI preferences
  inherit from personal defaults. Do not configure them here.
- Repository skills live in `.agents/skills/`: use
  [recollect-doc-router](.agents/skills/recollect-doc-router/SKILL.md) for task
  authority/current-state routing and
  [recollect-doc-maintainer](.agents/skills/recollect-doc-maintainer/SKILL.md)
  for documentation changes, audits and lifecycle reconciliation. They follow
  the docs; they do not add approval or delegation requirements.
- Do not rely on shared home agents, home profiles, or parent workspace config
  for Recollect's project setup. See [.codex/README.md](.codex/README.md).
- See [development tooling](docs/runbooks/development-tooling.md) for the host
  files and how to verify them.
