# Repository Governance Contract

Status: accepted

## Source

[ADR 0001](../adr/0001-repository-governance.md) and the user's approved setup plan.

## Contract

- Root instructions route meaningful work through the documented authority,
  epic, execution, validation, and closeout lifecycle.
- Each slice ID is unique across epics. A pack filename equals that ID and its
  owning epic contains the slice. Pack and slice statuses match.
- Packs declare `Work type: governance` or `Work type: product`. Governance
  covers repository docs and developer tooling only; runtime/application work
  is product work, even when small.
- Required packs precede implementation. `planned`, `in-progress`, and
  `blocked` packs live in active; `shipped` packs live in archive. Indexes must
  match actual files and statuses.
- Product packs cannot enter implementation or ship while foundation docs are
  pending. Pending sources cannot be listed under Governing Sources.
- Each shipped pack records all closeout fields and concrete validation
  evidence. Version/commit may explicitly be N/A or uncommitted.
- Project Codex configuration contains Context7, the Rust Analyzer/Graft
  developer tools of [ADR 0019](../adr/0019-rust-semantic-developer-navigation.md),
  and six agent registrations.
  Role files define names, descriptions, and instructions. Personal execution
  preferences remain inherited. Delegation requires explicit user request.
- OpenCode v2 has equivalent repository-local configuration in `opencode.json`
  and six Markdown roles in `.opencode/agents/`. Preserve role descriptions and
  instructions from `.codex/agents/`: `default` is the primary agent and the
  five specialist roles are subagents invoked only at the user's request.
  Both hosts use the same root `AGENTS.md`, `.agents/skills/` and CodeGraph CLI.
  The configured development MCPs are Context7 on both hosts plus secret-free
  local stdio chrome-devtools on OpenCode only for browser control. Codex
  also includes the repository-local Rust Analyzer and Graft stdio servers.
  Their launchers enforce the [navigation contract](rust-semantic-developer-navigation.md).
  Keep credentials and
  personal model, provider, permission and UI settings out of project files.
- Claude Code has equivalent repository-local configuration: root `CLAUDE.md`
  imports `AGENTS.md`; `.mcp.json` carries the Codex Context7, Rust Analyzer
  and Graft stanzas; `.claude/agents/` holds the five specialist roles with the
  `.codex/agents/` descriptions and instructions, invoked only at the user's
  request; the main session is `default`. `.claude/skills/` links to the shared
  `.agents/skills/` entrypoints instead of copying them. `.claude/settings.json`
  only denies the Rust Analyzer tools Codex omits from `enabled_tools`; project
  MCP approval and other permissions remain the user's.
- Repo-local `recollect-doc-router` and `recollect-doc-maintainer` skills live
  in `.agents/skills/`. The router identifies applicable authority, ownership,
  current evidence and decision gaps. The maintainer audits or updates affected
  docs and reconciles lifecycle records within the user's requested scope.
  Both use the existing documentation lifecycle and templates, preserve normal
  implicit invocation, and require no new MCP or home-level configuration.
- Secrets are inherited via named environment variables, not checked-in values.
- `references/cognee/`, `references/agent-memory-atlas/` and
  `references/claude-mem/` stay separate and ignored. Documentation maintenance
  must not modify those checkouts or execute their tests.
- Current status belongs in the roadmap and operating behavior in runbooks.
  Remove superseded proposals, duplicate reports and unused illustrations when
  their selected behavior is recorded in current authority/delivery records.
  Historical citations use exact Git revisions; preserve accepted authority,
  required shipped packs, frozen baselines and unfinished acceptance.
- `./scripts/validate.sh` runs offline using Bash and Python 3.11+, validates
  governance, and tests the checker against disposable fixtures. A failed
  check exits nonzero with the offending file and reason.

## Acceptance

Run the validator on the actual repository and exercise valid lifecycle states
plus invalid ownership, IDs, statuses, indexes, source authority, and config.
Verify local Codex configuration discovery and a read-only Context7 request
separately; network verification is not part of the offline validation command.
Compare Cognee's Git state and tracked-file content before and after setup.
For OpenCode setup, verify role parity, project configuration/agent/skill
discovery and a read-only Context7 call through the installed host separately.
For Claude Code setup, verify the same role parity and successful read-only
calls through each `.mcp.json` server separately from host discovery.
Neither configuration discovery nor tool listing alone proves a successful call.
For skill changes, validate the entrypoints and their local references, check
Codex discovery separately, and exercise routing/maintenance against actual
governing documents. Structural validation does not prove model behavior.

## Explicit Deferrals

Automatic release/version policy and speculative new developer integrations
remain deferred. The Rust product, Git remote and GitHub Pages workflow already
exist; changing their runtime or publishing is outside a documentation cleanup.
The checker enforces structure and consistency, not the truth of prose or runtime.
