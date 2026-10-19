@AGENTS.md

## Claude Code

The import above keeps `AGENTS.md` the single shared instruction source. Claude
Code's equivalents of the Codex setup are:

- `.mcp.json`: Context7, Rust Analyzer and Graft with the Codex stanzas'
  commands. `.claude/settings.json` denies the Rust Analyzer tools that Codex
  leaves out of `enabled_tools`.
- `.claude/agents/`: explorer, architect, worker, qa and reviewer, carrying the
  `.codex/agents/` descriptions and instructions. The main session is the
  `default` role. Delegate only when the user explicitly requests it.
- `.claude/skills/`: links to the shared `.agents/skills/` entrypoints; edit
  those, not copies.

See [development tooling](docs/runbooks/development-tooling.md) for setup and
verification.
