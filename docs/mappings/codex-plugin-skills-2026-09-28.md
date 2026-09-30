# Codex memory skills and OpenCode MCP rendering evidence, 2026-09-28

Source: official Codex repository (`/openai/codex`) and official OpenCode v2
docs (`/websites/opencode_ai_v2`, `/websites/opencode_ai_plugins`) via Context7;
installed Codex 0.157.1 and OpenCode v2.0.18; `crates/agent` unit proof.
Observation date: 2026-09-28. Confidence: observed-once for generated-bundle
proof; inferred for host-side skill discovery and OpenCode bridge interop.

## What was established

- Codex skills are subdirectories containing `SKILL.md` with YAML frontmatter
  (`name` max 64 chars, `description`), discovered from plugin skill roots; a
  plugin declares its root with the `skills` key in `plugin.json`
  (verified against `codex-rs/skills/src/parser.rs`,
  `codex-rs/ext/skills/src/host_roots.rs`, and an installed OpenAI `documents`
  plugin carrying `"skills": "./skills/"`).
- The generated `recollect-capture` bundle keeps its existing local marketplace
  shape and gains `skills/recollect-memory/SKILL.md` plus the `skills` key. No
  published marketplace, OAuth, or plugin rename is involved.
- OpenCode v2 runs project `mcp.servers` entries of `type: local` over stdio
  with a command array and environment map, matching the existing
  `recollect-mcp-bridge` stdio transport, so `mcp-config opencode` renders that
  shape from the same fixed Brain/directory/profile inputs.
- Unit proof: `cargo test --locked -p recollect-agent --lib` passes 10/10,
  including the new bundle test (frontmatter, skill references, unchanged hooks
  and marketplace files, refresh regeneration) and the OpenCode rendering test
  (local stdio shape, `configured_only`, no bearer material). No personal host
  configuration was touched.

## Blind spots and limits

- No live Codex run proving skill discovery or a scoped memory call through the
  new bundle; the installed personal Codex profile was deliberately left alone.
- No live OpenCode run against the bridge; OpenCode session capture was not
  attempted (different event/plugin system, needs its own contract and fixture).
- The Docker capture-hosts fixture was not rebuilt in this change.

## Next verification needed

Run the real-host skill proof (generated bundle registered in an isolated
Codex home, skill discovery plus `workspace.list` through the MCP tools) and a
live OpenCode stdio bridge handshake before shipping the owning pack. OpenCode
session capture belongs to a follow-up slice with its own host-adapter contract.

## Live proof, 2026-09-28/29

Paired a fresh device (`570a6b44`) with one macOS approval; `whoami`, `brains`,
`capture setup`, `capture run`, and every bridge load after were silent.
`capture setup codex` in `/Users/mike/devops/customer/SWEG` generated the full
bundle (hooks, `skills/recollect-memory/SKILL.md`, `skills` key, marketplace).
Findings along the way, all repaired and re-proven in this change:

- A same-name direct-HTTP `mcp_servers.recollect` project entry collides with
  the managed stdio override; the launch now refuses with guidance before any
  side effect (unit proof plus the live refusal).
- `register_codex_plugin` swallowed Codex's reason; it now surfaces the host's
  message (this exposed a stale marketplace registration from an earlier
  device profile, which was removed).
- Installed Codex 0.157.1 shows `recollect-capture@recollect-capture`
  installed and enabled; `SessionStart` and `UserPromptSubmit` hooks fired and
  completed; 3 events delivered, 1 publication, 0 pending.
- The model turn itself was blocked by empty OpenAI workspace credits
  (external, not product behavior). Capture still published, and autonomous
  learning accepted claim `85d94da7` from the session minutes later.
- The memory leg was driven mechanically through the same stdio bridge the
  skill describes: `start_task` with fresh SWEG inventory (80 checkouts),
  `begin` write operation, `contribute` (rejected bad kind vocabulary and
  missing supports with explicit errors first), then `recall` returning claim
  `89947f19` with `review_proposed` qualification and provenance to the
  live-captured Codex prompt.
- The SWEG project config was restored byte-identical (md5 verified).

Remaining open: a model-driven skill invocation through Codex (needs OpenAI
credits) and a live OpenCode bridge handshake. Confidence: verified for
everything above; observed-once for the exact prompt counts.
