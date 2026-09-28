# OpenCode project setup

Status: in-progress
Owning epic: `docs/roadmap/epics/developer-tooling.md`
Work type: governance

## Summary

- Goal: OpenCode can use Recollect's existing Codex project guidance and tools.
- Non-goals: Product MCP/plugin development, customer workspace setup, provider
  migration, personal settings, application changes, deployment or publishing.
- Delivery shape: Local host configuration, role definitions and documentation.

## Governing Sources

- [ADR 0001](../../../adr/0001-repository-governance.md)
- [Repository governance contract](../../../contracts/repository-governance.md)
- [Developer tooling epic](../../epics/developer-tooling.md)

## Scope

- In scope: OpenCode v2 project config, six equivalent roles, shared instructions
  and skills, Context7, existing CodeGraph access, startup and verification guide.
- Out of scope: Home configuration, extra MCP servers, skill duplication,
  new automatic delegation, product runtime and reference checkout changes.
- Blockers: None; user explicitly authorized host parity on 2026-09-28.

## Surface and Interface Changes

- Interfaces: Root `opencode.json`; `.opencode/agents/*.md`; `opencode` from the
  repository. Native v2 `mcp.servers` configuration, local stdio Context7.
- Storage: N/A; no product persistence change.
- Ownership: Developer tooling. Codex definitions remain supported and unchanged.

## Data and Authority

- Inputs: Existing `.codex/config.toml`, `.codex/agents/*.toml`, root `AGENTS.md`
  and the two `.agents/skills/` entrypoints.
- Authority: Shared governance and role instructions, with equivalent host syntax.
- Blind spots: Host discovery does not prove every model follows instructions;
  no product memory connection or session capture is implied.

## States and Edge Cases

- Loading: OpenCode discovers project config, root instructions and shared skills.
- Empty: No new provider selection; inherit the user's existing OpenCode defaults.
- Error: Invalid host config or missing role/skill fails acceptance.
- Blocked: Context7 lookup failure is reported separately from config discovery.
- No-access: Environment key is optional for supported unauthenticated Context7;
  never copy credential values into project configuration or evidence.
- Duplicate or replay: Use existing skill directory, not copied skill bodies;
  existing host files must not be overwritten without inspecting them.
- Stale data: Reopen sessions to load project changes; compare role bodies against
  Codex whenever either representation changes.
- Reconciliation divergence: Keep the six descriptions and instructions identical;
  only host-specific frontmatter and primary/subagent mode differ.

## Integrations and Runtime Inputs

- Providers: Installed OpenCode v2.0.18, Node/npx, existing Context7 stdio package.
- Environment: `CONTEXT7_API_KEY` may be inherited by the MCP subprocess.
- Secrets: No secret material in checked-in files or proof output.
- Failure handling: Use the host's normal MCP timeouts and reconnect behaviour;
  record actual tool-call failure rather than claiming config as connectivity.

## Tests and Acceptance

- Automated: JSON/TOML/Markdown parity checks, installed-host config/agent/skill
  discovery from root and nested directory, read-only Context7 lookup,
  `./scripts/validate.sh`, and `git diff --check`.
- Manual: Document startup and explicit specialist/skill use.
- Acceptance: Six role definitions and two shared skills are discoverable, root
  instructions are available, and Context7 completes a read-only call through
  OpenCode. Codex setup and unrelated dirty work remain intact.

## Closeout

- Planned: OpenCode parity for the existing repository developer setup.
- Shipped: Pending installed-host verification.
- Not shipped: Product changes, global settings, commit, push or deployment.
- New blockers: None.
- Docs updated: Governance authority and developer-tooling lifecycle; setup guide
  and dated host evidence will accompany verification.
- Validation: Pending.
- Version: N/A; developer configuration only.
- Commit: Uncommitted.
