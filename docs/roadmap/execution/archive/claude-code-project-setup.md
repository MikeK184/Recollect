# Claude Code project setup

Status: shipped
Owning epic: `docs/roadmap/epics/developer-tooling.md`
Work type: governance

## Summary

- Goal: Claude Code can use Recollect's existing project guidance and developer tools.
- Non-goals: Product plugin or memory connection, personal model/permission/approval
  settings, home configuration, application changes, deployment or publishing.
- Delivery shape: Local host configuration, role definitions and documentation.

## Governing Sources

- [ADR 0001](../../../adr/0001-repository-governance.md)
- [Rust semantic navigation ADR](../../../adr/0019-rust-semantic-developer-navigation.md)
- [Repository governance contract](../../../contracts/repository-governance.md)
- [Rust/Graft navigation contract](../../../contracts/rust-semantic-developer-navigation.md)
- [Developer tooling epic](../../epics/developer-tooling.md)

## Scope

- In scope: Root `CLAUDE.md` importing `AGENTS.md`, project `.mcp.json` with
  Context7, Rust Analyzer and Graft, five specialist subagents, links to the two
  shared skills, a project deny-list matching Codex `enabled_tools`, docs.
- Out of scope: chrome-devtools (Claude Code has its own browser tools), MCP
  pre-approval, skill copies, a `default` subagent, new automatic delegation,
  governance-lint changes and reference checkout changes.
- Blockers: None; the user requested Claude Code parity on 2026-10-18.

## Surface and Interface Changes

- Interfaces: `CLAUDE.md`; `.mcp.json` (`mcpServers` stdio stanzas);
  `.claude/agents/{explorer,architect,worker,qa,reviewer}.md`;
  `.claude/skills/recollect-doc-{router,maintainer}` symlinks;
  `.claude/settings.json` `permissions.deny` for four Rust Analyzer tools.
- Storage: N/A; no product persistence change.
- Ownership: Developer tooling. Codex and OpenCode definitions are unchanged.

## Data and Authority

- Inputs: `.codex/config.toml`, `.codex/agents/*.toml`, root `AGENTS.md`, the
  `.agents/skills/` entrypoints and current Claude Code documentation
  (code.claude.com memory, mcp, sub-agents, skills and permissions pages).
- Authority: Shared governance and role instructions with host-specific syntax.
- Blind spots: Stdio proof used the exact `.mcp.json` commands outside the
  Claude Code host; in-host `/mcp` approval and model behavior are not proven.

## States and Edge Cases

- Loading: Claude Code reads `CLAUDE.md`, which imports `AGENTS.md` once.
- Empty: No model or permission defaults; subagents inherit the main model.
- Error: Missing navigation install makes the launchers exit with setup guidance.
- Blocked: Unapproved project MCP servers stay off until the user approves them.
- No-access: `CONTEXT7_API_KEY` defaults to empty; keyless Context7 calls work.
- Duplicate or replay: Skills are symlinks, not copies; roles are generated
  from the Codex TOML bodies.
- Stale data: Reopen sessions after config changes; recompare role bodies when
  a Codex role changes.
- Reconciliation divergence: Descriptions and instructions match Codex exactly;
  only frontmatter and explorer/reviewer `disallowedTools` differ.

## Integrations and Runtime Inputs

- Providers: Node/npx Context7 4.4.2, repository-local rust-analyzer-mcp 0.4.0
  and Graft 0.21.1 under macOS sandbox-exec.
- Environment: Optional `CONTEXT7_API_KEY`; `CLAUDE_PROJECT_DIR` when set by the host.
- Secrets: No secret material in checked-in files or proof output.
- Failure handling: Host MCP timeouts and reconnection; record failed calls
  rather than treating registration as connectivity.

## Tests and Acceptance

- Automated: Role parity against `.codex/agents/`, JSON parsing, stdio
  initialize/tools/list/read-only call for each `.mcp.json` server,
  `./scripts/validate.sh` and `git diff --check`.
- Manual: In Claude Code, approve the servers, then check `/mcp`, `/memory`,
  `/agents` and the two skills.
- Acceptance: Five roles match Codex, both skills resolve through the links,
  and each server completes a read-only call through its configured command.

## Closeout

- Planned: Claude Code parity for the existing repository developer setup.
- Shipped: `CLAUDE.md`, `.mcp.json`, five roles, two skill links and the
  Rust Analyzer deny-list. Context7 `resolve-library-id` (tokio), Rust Analyzer
  `rust_analyzer_symbols` on `crates/server/src/db.rs` launched from
  `crates/server`, and Graft `graft_check_freshness` succeeded through the
  exact `.mcp.json` commands.
- Not shipped: In-host `/mcp` connection and model-driven calls, because no
  `claude` CLI was on PATH in the verification shell; chrome-devtools by choice.
- New blockers: None for this slice.
- Docs updated: This pack, owning epic, archive index, repository governance
  contract, development tooling runbook and `AGENTS.md`.
- Validation: Role bodies and descriptions match the Codex TOML exactly;
  `./scripts/validate.sh` and `git diff --check` passed.
- Version: N/A: no release requested.
- Commit: Uncommitted.
