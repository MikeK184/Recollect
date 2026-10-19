@AGENTS.md

## Claude Code

The import above keeps `AGENTS.md` the single shared instruction source.

- `.mcp.json`: Context7, Rust Analyzer and Graft, with the same commands as
  `.codex/config.toml`. `.claude/settings.json` denies the Rust Analyzer tools
  that Codex leaves out of `enabled_tools`.
- `.claude/agents/`: the role definitions for explorer, architect, worker, qa
  and reviewer, shared with Codex and OpenCode. The main session is `default`.
  Delegate only when the user explicitly requests it.
- `.claude/skills/`: links to the shared `.agents/skills/` entrypoints; edit
  those, not copies.

See [development tooling](docs/runbooks/development-tooling.md) for setup and
verification.
