# OpenCode project setup

Status: shipped
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
  and skills, Context7, secret-free chrome-devtools MCP for browser control,
  existing CodeGraph access, startup and verification guide.
- Out of scope: Home configuration, further MCP servers beyond Context7 and
  chrome-devtools, skill duplication,
  new automatic delegation, product runtime and reference checkout changes.
- Blockers: None; user explicitly authorized host parity on 2026-09-28 and
  OpenCode chrome-devtools browser control on 2026-09-29.

## Surface and Interface Changes

- Interfaces: Root `opencode.json`; `.opencode/agents/*.md`; `opencode` from the
  repository. Native v2 `mcp.servers` configuration, local stdio Context7 and
  secret-free local stdio chrome-devtools (`npx -y chrome-devtools-mcp@latest`
  with `--isolated` and `--no-usage-statistics`).
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

- Providers: Installed OpenCode v2.0.21, Node/npx, existing Context7 stdio package,
  `chrome-devtools-mcp@latest` stdio package, and local Chrome for browser control.
- Environment: `CONTEXT7_API_KEY` may be inherited by the MCP subprocess.
  chrome-devtools needs no project secret; `CHROME_DEVTOOLS_MCP_NO_UPDATE_CHECKS`
  may be set locally to silence update checks without changing checked-in files.
- Secrets: No secret material in checked-in files or proof output.
- Failure handling: Use the host's normal MCP timeouts and reconnect behaviour;
  record actual tool-call failure rather than claiming config as connectivity.

## Tests and Acceptance

- Automated: JSON/TOML/Markdown parity checks, installed-host config/agent/skill
  discovery from root and nested directory, read-only Context7 lookup,
  chrome-devtools discovery plus a read-only `list_pages` call,
  `./scripts/validate.sh`, and `git diff --check`.
- Manual: Document startup and explicit specialist/skill use.
- Acceptance: Six role definitions and two shared skills are discoverable, root
  instructions are available, Context7 completes a read-only call through
  OpenCode, and chrome-devtools connects with a successful `list_pages` call.
  Codex setup and unrelated dirty work remain intact.

## Closeout

- Planned: OpenCode parity for the existing repository developer setup plus
  secret-free chrome-devtools browser control.
- Shipped: OpenCode 2.0.21 discovers all six equivalent roles, both shared skills and root instructions from the root and web directory. An actual model session completed Context7 React resolution and chrome-devtools list_pages.
- Not shipped: Product MCP/capture, personal provider defaults and marketplace publication remain outside this developer-setup slice.
- New blockers: None for this slice.
- Docs updated: This pack, owning epic, active/archive and epic indexes, handoff, relevant runbooks and [final acceptance evidence](../../../mappings/desktop-final-acceptance-2026-10-01.md).
- Validation: Six role bodies match Codex exactly; root/nested installed-host discovery and both actual MCP calls passed. The one-off proof selected a working OpenAI model in its dedicated process; the user's unavailable custom default was not changed. Final repository checks are recorded in the linked evidence.
- Version: N/A: no release requested.
- Commit: Uncommitted.
