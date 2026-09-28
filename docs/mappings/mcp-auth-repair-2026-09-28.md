# SWEG direct Codex authentication repair

Observed: 2026-09-28
Confidence: observed-once

## Sources and Method

- User's actual Codex 0.157.1 screenshot reported Recollect failed with zero tools.
- Targeted inspection of SWEG's `.codex/config.toml`, current local server, and
  installed `/opt/homebrew/bin/codex`; no customer infrastructure operations.
- Official [MCP documentation](https://learn.chatgpt.com/docs/extend/mcp) and
  [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
  establish that `bearer_token_env_var` names a variable and that a local HTTP
  `http_headers_helper` may return authorization-header JSON. Read on 2026-09-28.
- Installed macOS `security help add-generic-password` specifies a final `-w`
  option to prompt for the password instead of putting it in command arguments.
- No new library dependency was introduced. Existing primary host/OS interfaces
  were verified directly rather than inferred from the earlier bridge fixture.
- Owned by [mcp-direct-auth-repair](../roadmap/execution/archive/mcp-direct-auth-repair.md).

## Observations

The access-token value had been pasted into `bearer_token_env_var`. That named
environment variable did not exist, nor was `RECOLLECT_MCP_TOKEN` exported.
The same credential successfully initialized the actual SWEG HTTP endpoint when
supplied correctly. The connection dialog had named the variable but omitted a
concrete credential handoff. Previous raw HTTP tests did not cover that gap.

The repair moved the authorization-header JSON into a named macOS Keychain
generic-password item and replaced only SWEG's Recollect credential line with
Codex's native header helper. The item is selected by the Brain's complete MCP
URL and the fixed `recollect` account. Readback succeeded; the token is absent
from project configuration. Parsed comparison verified other host settings were
unchanged. No Recollect launcher, global Codex settings or shell profile changed.

The setup dialog now defaults to this Keychain flow on macOS for Codex and
offers environment authentication explicitly. Keychain JSON is copied only on
the user's action and entered at the native hidden password prompt. The
environment path distinguishes the variable name from the token and supplies a
hidden-input/export/host-launch block. Closing still clears the token.

## Validation

- Installed **codex-cli 0.157.1**, started in the actual SWEG directory, consumed
  the repaired project configuration and Keychain credential with
  `RECOLLECT_MCP_TOKEN` explicitly absent. Recollect was required at startup.
  A real `workspace.list` call completed with structured data containing **one
  workspace and 79 repositories**, and its result reached the next model turn.
- A synthetic Responses endpoint bound to loopback selected exactly that
  read-only tool. Other MCP servers and hooks were disabled for this invocation;
  customer instruction loading and memory generation were disabled. No customer
  content was sent to a paid/external model and no real Brain records were added.
  Report: `.cache/mcp-auth-repair-20260928/codex-proof.json`.
- Initial fixture invocation ignored the user config, losing the project trust
  entry and producing `invalid transport` before startup. It was corrected to
  preserve normal trust/config loading while disabling unrelated tools only for
  that invocation. The first successful run exposed a missing synthetic models
  endpoint; the final fixture serves it and explicitly checks the structured
  result, rather than treating a completed error response as success.
- Three focused browser cases passed: direct token/Keychain config/HTTP recall
  and revocation, name/URL discovery without execution, and both coding-host
  setup paths. The unrelated opt-in Context7 live probe was skipped. The first
  Agents test used a heading as a button locator; the corrected existing
  **Connect coding agent** locator passed on its targeted rerun.
  Logs: `.cache/mcp-auth-repair-20260928/ui.log` and `ui-agent.log`.
- Frontend design/typecheck/build, `git diff --check`, and
  `./scripts/validate.sh` (32 governance tests) passed.
- Root Compose rebuilt and deployed image **385793c54044**. API readiness at
  `/health/ready` passed and the worker is running. Before/after recorded inventory is equal: the same
  seven Brain IDs, 16 sources, 17 versions, 11 claims and 17 claim revisions.
  Owned UI databases were cleaned; none remain. Evidence is in this repair's
  `before.json`, `after.json`, `deploy.log` and `governance.log` cache files.
- The live SWEG dialog shows the Keychain helper and hidden-prompt command;
  inspected without creating a second real credential. Screenshot:
  `.cache/mcp-auth-repair-20260928/live-keychain-setup.png`.

## Translation and Limits

The actual SWEG credential/configuration path is proven through the installed
host. An already-open failed Codex process must be restarted or resumed in a
fresh process to load the repaired configuration. That process was not
interrupted. The proof does not claim every tool, meaningful memory recall,
automatic session capture, packaged plugin delivery or OAuth login is verified.
The six earlier desktop acceptance packs remain open.

Local and uncommitted; version N/A. Earlier direct-connection evidence remains
historical and is supplemented by this actual host repair.
