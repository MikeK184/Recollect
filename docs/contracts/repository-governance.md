# Repository Governance Contract

Status: accepted

## Source

[ADR 0001](../adr/0001-repository-governance.md), including its 2026-10-19
amendment, and the user's approved setup plan.

## Contract

- Root instructions route meaningful work through the documented authority,
  epic, execution, validation, and closeout lifecycle.
- Each slice ID is unique across epics. Slice statuses are `planned`,
  `in-progress`, `blocked` and `shipped`. In-progress and shipped slices cannot
  carry `needs-adr`, `needs-contract` or `mapping-only` and must cite accepted
  authority.
- Every slice has an Execution value: `pack` (an active pack
  `docs/roadmap/execution/active/{slice-id}.md` exists; only for planned,
  in-progress or blocked slices), `inline: <acceptance; validation>` (no pack)
  or `archived` (shipped; the pack was removed at closeout and is retained in
  Git history).
- A pack is required only for risky work: a schema or migration change with
  restore or compatibility impact; a new external integration or a paid or
  frozen evaluation campaign (cost approval and frozen input hashes); a
  security, credential or authz change no accepted contract fully specifies;
  or multi-session work with open decisions. Any behavior change still updates
  the governing contract in the same change.
- A pack filename equals its slice ID, its owning epic contains the slice and
  their statuses match. Pack statuses are `planned`, `in-progress` and
  `blocked`; there is no `shipped` pack. A pack requires Summary (Goal,
  Non-goals), Governing Sources, Scope (In scope, Out of scope, Blockers),
  Decisions, and Acceptance (Automated, Manual, Acceptance), and may add more
  sections. The active index must match actual files and statuses and lists
  packs and statuses only. Pending sources cannot be listed under Governing
  Sources.
- Closeout: run focused validation and `./scripts/validate.sh`; update affected
  contracts and runbooks; set the slice to `shipped` with Execution `archived`
  and a one-line Summary naming the validation evidence; delete the pack;
  update the active and epic indexes. Deferrals and failures are recorded in
  the slice Summary or a successor slice, never hidden. Slices record neither a
  version nor a commit.
- `AGENTS.md` is the only shared instruction prose. Role definitions live once in
  `.claude/agents/`; Codex (`.codex/agents/`) and OpenCode (`.opencode/agents/`)
  keep stubs that reference them, and the main session is `default`. Specialist
  roles run only at the user's request. Each host declares its MCP servers once
  in its own file (`.codex/config.toml`, `.mcp.json`, `opencode.json`): Context7
  on all hosts, the repository-local Rust Analyzer and Graft stdio servers on
  Codex and Claude Code with identical commands, and secret-free chrome-devtools
  on OpenCode. The
  [development tooling runbook](../runbooks/development-tooling.md) lists the
  files and launcher rules. Keep credentials and personal model, provider,
  permission and UI settings out of project files.
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
- Current status belongs in the roadmap (`docs/roadmap/epics/index.md`) and
  operating behavior in runbooks. Remove superseded proposals, duplicate
  reports and unused illustrations when their selected behavior is recorded in
  current authority and the epic slice rows. Historical citations, including
  shipped packs and long evidence ledgers, use exact pinned Git revisions;
  preserve accepted authority, frozen baselines, paid-run approvals, frozen
  input hashes, failure results and unfinished acceptance. Mappings carry an
  `Observed:` line and keep paid or frozen campaigns as a compact receipts
  table; the checker warns above 5,000 words.
- `./scripts/validate.sh` runs offline using Bash and Python 3.11+, validates
  governance, and tests the checker against disposable fixtures. A failed
  check exits nonzero with the offending file and reason.

## Acceptance

Run the validator on the actual repository and exercise valid lifecycle states
(including `pack`, `inline:` and `archived` rows and packs with extra sections)
plus invalid ownership, IDs, statuses, indexes, source authority and mappings
without an `Observed:` line.
Verify local Codex configuration discovery and a read-only Context7 request
separately; network verification is not part of the offline validation command.
Compare Cognee's Git state and tracked-file content before and after setup.
For each host, verify configuration and role discovery and a successful read-only
call through each MCP server separately.
Neither configuration discovery nor tool listing alone proves a successful call.
For skill changes, validate the entrypoints and their local references, check
Codex discovery separately, and exercise routing/maintenance against actual
governing documents. Structural validation does not prove model behavior.

## Explicit Deferrals

Automatic release/version policy and speculative new developer integrations
remain deferred. The Rust product, Git remote and GitHub Pages workflow already
exist; changing their runtime or publishing is outside a documentation cleanup.
The checker enforces structure and consistency, not the truth of prose or runtime.
