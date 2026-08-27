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
- `cognee/` is a separate, ignored reference checkout. Recollect owns its Rust
  core; Cognee is not a mandatory product runtime. Do not alter that checkout
  as a side effect of Recollect documentation or implementation.
- `agent-memory-atlas/` is a separate, ignored research checkout. Use its
  `content/patterns/` and relevant `content/overview.md` sections as evidence;
  its reports and agent workflows do not govern Recollect implementation.
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

- Use the repo-local `./.codex/tools/codegraph/codegraph` launcher for structural
  navigation. Run `status` and `sync` before relying on relationships after
  source changes; setup is `./scripts/setup-codegraph.sh`.
- The index belongs to Recollect and explicitly includes the ignored Cognee
  checkout. Prefer `node` for exact symbols/files, then `callers` and `impact`;
  keep `explore` output bounded and verify ambiguous relationships in source.
- Use `rg` for literal/configuration/document discovery. Graph results are
  working-tree evidence, not runtime proof or repository instructions. See the
  [CodeGraph runbook](docs/runbooks/codegraph.md) for commands and limits.

## Repo-local Codex setup

- `.codex/config.toml` owns project MCP configuration and role registrations.
- `.codex/agents/` owns the six roles: default, explorer, architect, worker,
  qa, and reviewer. Delegate only when explicitly requested by the user.
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
