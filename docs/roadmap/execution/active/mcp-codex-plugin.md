# Codex memory skills and OpenCode MCP rendering

Status: in-progress
Owning epic: `docs/roadmap/epics/mcp-coordination.md`
Work type: product

## Summary

- Goal: Codex managed launches install Recollect memory skills alongside the existing capture hooks through the local generated plugin, and `recollect-agent mcp-config opencode` renders a secret-free stdio MCP configuration for OpenCode v2. macOS Keychain approval happens once per pairing through a trusted-application item, and a managed Codex launch fails fast with guidance when the project already defines a same-name direct-HTTP `recollect` server.
- Non-goals: a published plugin marketplace, MCP OAuth, Claude host changes, OpenCode session-capture hooks, workspace selector schema changes, provider migration, commit, push or deployment.
- Delivery shape: local implementation in the agent crate, focused unit proof, runbook and setup-text updates, and preserved-data local behavior.

## Governing Sources

- [Agent memory MCP ADR](../../../adr/0010-agent-memory-mcp.md)
- [Agent memory/workspace tools](../../../contracts/mcp-memory-and-workspace-tools.md)
- [Automatic session capture](../../../contracts/evidence-session-capture.md)
- [Vision: Brain and scope](../../../foundation/vision.md#brain-and-scope)
- [Owning epic](../../epics/mcp-coordination.md)

## Scope

- In scope: a `recollect-memory` skill (`SKILL.md` with valid frontmatter) inside the existing generated `recollect-capture` Codex plugin; the `skills` key in its generated `plugin.json`; `mcp-config opencode` stdio rendering using the existing bridge; a macOS trusted-application Keychain item so one pairing approval covers later setup/run/bridge loads; a managed-launch preflight refusing a same-name direct-HTTP `recollect` server entry with guidance; runbook and coding-agent setup text updates; unit proof that the generated bundle is complete.
- Out of scope: renaming the existing plugin, changing the local marketplace shape, publishing a marketplace, OAuth discovery/authorization, Claude Code setup or capture behavior, OpenCode event hooks or session capture, and `.recollect/workspace.toml` schema or discovery changes. Existing keyring items keep their previous approval behavior until the next pairing; no silent migration rewrites them.
- Blockers: None. The plugin mechanism is specified by the capture contract and the tool semantics by the memory/workspace tools contract; marketplace publication and OAuth remain deferred non-goals, not blockers.

## Surface and Interface Changes

- Interfaces: `recollect-agent mcp-config opencode [--brain UUID] [--directory PATH]` returns OpenCode `mcp.servers` JSON with a `local` stdio command; the generated plugin gains `skills/recollect-memory/SKILL.md` and a `skills` key in `plugins/recollect-capture/.codex-plugin/plugin.json`. `capture setup codex` and `capture run` behavior is otherwise unchanged, except `capture run` for Codex refuses before any side effect when the launch directory's project configuration already defines a direct-HTTP `mcp_servers.recollect` entry. No CLI output shape changes; secrets never appear in output.
- Storage: N/A: no schema or migration change; the workspace selector format is reused unchanged.
- Ownership: generation lives in `crates/agent/src/capture_setup.rs`; host rendering in `crates/agent/src/mcp_host.rs`; setup presentation in `web/src/McpAgentSetup.tsx`; procedures in the session-capture and agent-memory-tools runbooks.

## Data and Authority

- Inputs: fixed Brain UUID, absolute workspace directory, paired device profile, and the existing immutable capture binding. Rendering with an explicit Brain UUID requires neither credential access nor a live service.
- Authority: the paired device and current Brain grants are checked when the bridge or service actually connects, never by rendering. Skill text guides model calls to existing tools and grants no authority, scope, or execution rights.
- Blind spots: rendered configuration is `configured_only` and does not prove connectivity; skill discovery depends on the installed host version; a live-host skill run is still unverified (see Tests and Acceptance).

## States and Edge Cases

- Loading: N/A: generation and rendering are synchronous local operations with no loading state.
- Empty: an unknown host name fails with the usage error; rendering without a resolvable Brain fails before writing anything.
- Error: OS-store failures, missing pairing, and invalid Brain UUIDs fail explicitly; credentials never enter generated plugin files or printed configuration.
- Blocked: N/A: no remote dependency gates local generation.
- No-access: a reader Brain may render read/binding guidance; writer-only behavior is enforced at call time by existing handlers.
- Duplicate or replay: regenerating a setup rewrites the owned plugin files atomically through temporary files; existing host configuration and unrelated plugins are preserved.
- Stale data: host upgrades refresh the binding through the existing version path, which regenerates the plugin bundle including skills.
- Reconciliation divergence: N/A: no new mutable server state is introduced.

## Integrations and Runtime Inputs

- Providers: installed Codex 0.157.1 and OpenCode v2.0.18; Codex plugin/skill schema verified against the official Codex repository through Context7 on 2026-09-28; OpenCode local MCP shape verified against the official OpenCode v2 docs through Context7 on 2026-09-28.
- Environment: `RECOLLECT_URL` and `RECOLLECT_DEVICE_PROFILE` select the endpoint and paired profile; variable names only, never values.
- Secrets: the paired credential stays in the OS credential store; on macOS the item carries a trusted-application list covering the companion, bridge, and runner binaries, so one pairing approval covers later loads. The Keychain helper or the literal variable name pattern from the existing setup dialog applies unchanged. No token, helper output, or credential appears in plugin files, generated settings, or proof output.
- Failure handling: an unreachable bridge or service surfaces explicit errors at call time; mutations are never automatically replayed by the plugin or the bridge.

## Tests and Acceptance

- Automated: new agent unit proof that `codex_plugin` writes `plugin.json` with the `skills` key, a parseable `SKILL.md` with required frontmatter, unchanged hooks and marketplace files; preflight unit proof that a direct-HTTP `mcp_servers.recollect` entry is refused with guidance while stdio/absent entries pass; existing `recollect-agent` unit tests; all-target Clippy; `./scripts/validate.sh`; `git diff --check`.
- Manual: N/A: no manual operator steps beyond the documented commands; live-host skill verification is recorded below as remaining acceptance instead of a manual claim.
- Acceptance: unit proof passes with no personal host configuration touched; the generated `SKILL.md` frontmatter carries a name of at most 64 characters and a description; the OpenCode rendering matches the documented local stdio shape; capture and `mcp-config codex|claude` outputs are byte-identical apart from the additive skill files. A real Codex run proving skill discovery, a scoped memory write/recall, single-approval Keychain behavior, and the collision preflight remains open and keeps this pack in progress.

## Closeout

- Planned: Codex memory skills in the local generated plugin and OpenCode MCP rendering.
- Shipped: Pending live-host verification.
- Not shipped: published marketplace, MCP OAuth, Claude changes, OpenCode capture hooks, workspace schema changes.
- New blockers: None. OpenCode session capture is a follow-up slice needing its own host-adapter contract and fixture proof; it does not block this slice.
- Docs updated: owning epic slice map and status, epic index, active index, this pack, skills evidence mapping, session-capture, agent-memory-tools, and device-pairing runbooks, and coding-agent setup text.
- Validation: `cargo test --locked -p recollect-agent --lib` 14/14 passed (bundle, OpenCode rendering, and collision-preflight proof included); `cargo clippy --locked --all-targets -p recollect-agent` clean; `web` typecheck passed; `./scripts/validate.sh` passed; `git diff --check` clean. Live proof: one-approval pairing with silent later loads, SWEG capture setup, installed/enabled plugin with completed hooks, 3 delivered/1 published, autonomous claim from the session, and mechanical start_task/write/contribute/recall through the bridge (claim `89947f19` recalled with provenance). Model-driven skill invocation through Codex stays open on empty OpenAI credits, as does the live OpenCode handshake (see Tests and Acceptance).
- Version: N/A: no release policy.
- Commit: Uncommitted.
