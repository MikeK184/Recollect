# Plugin-managed memory implementation evidence — 2026-10-01

Status: delivered local evidence

## Scope and method

The user approved the full integration described in
[ADR 0018](../adr/0018-plugin-managed-agent-memory.md) and the
[execution pack](../roadmap/execution/archive/mcp-plugin-session-memory.md).
This mapping separates interface research, local implementation and actual host
proof. Earlier desktop/plugin closeouts prove the previous integration only.

## External interfaces

- Context7 resolved `/topoteretes/cognee-integrations` and its current
  [Claude](https://github.com/topoteretes/cognee-integrations/blob/main/integrations/claude-code/README.md)
  and [Codex](https://github.com/topoteretes/cognee-integrations/blob/main/integrations/codex/README.md)
  integrations. They document HTTP hook clients, prompt context and detached workers.
- [OpenAI packaging documentation](https://developers.openai.com/plugins/build/plugins)
  documents bundled MCP/hook paths, `PLUGIN_ROOT`/`PLUGIN_DATA` and Claude-compatible
  root/data variables. Non-managed hooks still require host trust. Installed Codex
  is 0.159.3; its hooks/plugins features report stable/enabled.
- Context7 resolved `/websites/opencode_ai_v2`. The
  [V2 migration](https://opencode.ai/v2/docs/build/plugins/migrate-v1) and
  [plugin API](https://opencode.ai/v2/docs/build/plugins/) specify `setup(ctx)`,
  session prompt/context/compaction hooks, tool hooks and native event subscription.
  V1 implementations do not run in V2. Installed OpenCode is 2.0.21.
- Official npm `@opencode/plugin`, `@opencode/schema`, `@opencode/client` and
  `@opencode/ai` 2.0.21 artifacts were inspected in the ignored interface cache.
  They establish actual event fields, per-session locations, full visible-text
  completion and MCP/skill transform shapes. No dependency code was added to the
  product runtime. OpenCode's `Plugin.define` returns its input unchanged.
- Failed guessed URLs: Cognee `integrations/codex/.codex-plugin/plugin.json` and
  `integrations/codex/hooks.json` returned 404; OpenAI `/docs/hooks` and OpenCode
  `/docs/v2/plugins/` did not resolve. The official pages above and installed
  interface artifacts supersede those guesses; no behavior depends on them.

## Implementation and intermediate checks (historical)

- Bundled `recollect-plugin` runtime, portable private storage, host session/task
  identity, scoped recall, existing capture/privacy delivery and explicit runner
  enablement are implemented locally. This is not yet a complete acceptance claim.
- Codex/Claude manifests now include native hooks, the packaged MCP adapter and a
  connect skill. A first native artifact built under the ignored plugin-dist cache.
  Packaging includes the runtime; a consumer does not need the source checkout or Cargo.
- Native OpenCode V2 adapter uses admitted session messages from
  `ctx.session.context`, documented tool hooks and event subscription. It excludes
  reasoning and derived first-party MCP results. Its installed-host proof passed
  with native MCP tools, as detailed below.
- Migration 030 admits OpenCode under the existing host capture constraints/RLS;
  managed internal observations retain their separate authority.
- Agent/server `cargo check` passed. Four targeted plugin/storage/session tests
  passed after the old skills-only validator was replaced with packaged-hook/MCP
  validation. Docs validation passed all 32 checker tests.
- The real HTTP/SQL/Secret Service fixture passes for Codex, Claude and OpenCode
  payloads: cited automatic recall, canonical capture, session isolation,
  duplicate delivery, revocation, default runner disabled, capture-disabled reads,
  session-end task closure and scope-preserving resume. This is runtime evidence,
  not native-host adapter acceptance.
- Ordinary installed Codex 0.154.0, Claude Code 2.1.270 and OpenCode 2.0.21
  passed automatic cited context before the first model tool call, an actual
  scoped workspace tool result and canonical prompt capture in the isolated Linux
  native fixture (`.cache/plugin-native-host-proof-10.log`). Codex and Claude use
  their marketplace installers; no `capture run` or Claude `--plugin-dir` is needed.
  The server and model endpoints are owned fixtures, not paid external models.
- The Codex 0.154.0 legacy MCP loader leaves `${CLAUDE_PLUGIN_ROOT}` unchanged in
  executable fields (its hook loader expands that variable). The private runtime
  pointer and quoted POSIX launcher solve this without a PATH installation. An
  earlier test also searched generated diagnostic code instead of actual tool
  output; it now checks successful scoped MCP output specifically.
- [OpenCode MCP documentation](https://opencode.ai/v2/docs/mcp-servers/) describes
  default Code Mode and the supported `codemode: false` alternative. In 2.0.21 the
  native proof observed all 22 tools connected while Code Mode's complete catalogue
  omitted them. The plugin exposes native MCP tools instead; the same host proof
  then executed a real workspace call. No host permission is bypassed by this setting.
- The optional runner flag passed real local stdio and private HTTP execution;
  reconnecting without the flag stopped only the plugin-owned runner while the
  independently hosted HTTP fixture stayed alive. Receipt/replay/scope assertions
  remain in that native test. Ordinary memory sessions started no runner.
- Agents setup now presents plugin install/connect as the normal path with direct
  HTTP and the optional runner under advanced options. Focused browser proof passed
  two cases with one optional Context7 case skipped; web typecheck/build passed.
  Owner visual acceptance and final regression remain open.
- Native fixture now includes pinned OpenCode CLI 2.0.21. Official OpenAI
  `openai/codex` Rust source at `rust-v0.154.0` is being used to investigate plugin
  loading. Guessed old `core/src/plugins.rs` and `ext/mcp/src/plugin` URLs returned
  404; the GitHub tree supplied the correct `core-plugins/src/loader.rs` and
  `ext/mcp/src/executor_plugin` paths.
- A macOS ARM release package was built with the native runtime and Enola extractor
  under `.cache/plugin-dist/darwin-arm64-20261001`. Later fixes require a fresh final
  artifact; this artifact is not an installed-owner or final release proof.
- Reliability audit added a hard eight-second hook-process deadline, delivery
  wake-up handoff, recovery of a committed capture binding after session-file
  interruption, and a gap when a new scope cannot acquire its matching capture
  binding. These later changes require their current regression results.
- Intermediate complete-fixture runs were red: a new learning fixture initially
  omitted Codex's `turn_id` (the runtime correctly withheld ambiguous content), and
  intermittent database/HTTP timeouts affected subsequent tests. They are not
  final acceptance. The clean rerun in `.cache/plugin-native-host-proof-11.log`
  passed all six tests in 44.12 seconds, including capture through autonomous
  acceptance, a cited learned claim in a fresh session, and fresh recall at the
  documented compaction hook boundary for all three host payload formats. Native
  host launch/capture is proved separately by the installed-host tests; this does
  not yet claim a host-initiated native compaction or semantic-quality benchmark.
- Read-only migration inspection now identifies the old managed-launch environment,
  named first-party MCP entries and legacy capture hooks in Codex/Claude config.
  Status/connection report only host, conflict kind and path, never configuration
  values. Hooks with a conflicting setup withhold duplicate capture and explain
  migration. OpenCode inspects its effective MCP entry in the native transform.
  Focused plugin tests passed five cases, including migration parsing, secret-free
  bundle validation, storage protection and session separation.
- The next clean run (`.cache/plugin-native-host-proof-12.log`) passed all seven
  tests in 52.63 seconds. It additionally proves learning after the source task
  closes, reader-only recall without capture authority, and an eight-second
  process deadline even when standard-input reading blocks outside async I/O.
  Later connection/queue changes are under a new test pass and are not included
  in this seven-test result.
- Run 13 passed eight tests in 58.58 seconds, adding failed-connection preservation
  and delayed delivery with the original device/Brain after reconnect. Run 14
  passed the same eight tests in 69.61 seconds with child-task separation, child
  scope changes and delayed tool completion added to all three adapter formats.
  Codex/Claude child identity is keyed by the native session/agent tuple; an
  unobserved child cannot borrow the root task for recall. This child regression
  drives real runtime processes with documented hook payloads, not native spawning.
  The Codex 0.154 source confirms shared root session IDs; Claude's current hooks
  reference and Context7 confirm separate `agent_id` and SubagentStart context.
- Agent/protocol tests and strict clippy passed after those fixes. The new
  OpenCode local erasure test exposed and fixed its omission from the inbox's
  privacy-fence host allowlist; erased content is refused on replay while an
  unrelated event remains usable. Docs validation passed all 32 cases; frontend
  typecheck/build passed, with the existing bundle-size advisory.
- A stricter installed-host proof now requires retained prompt **and reply**
  sources, plus exclusion of derived MCP tool results. Codex and Claude passed;
  OpenCode failed because no visible reply source was retained. Earlier prompt-only
  passes do not prove reply coverage. Native event/cleanup handling is being
  investigated through metadata-only fixture diagnostics. OpenCode 2.0.21
  delivered no lifecycle events to this subscription during the owned CLI run,
  while its documented context API exposed the completed reply at cleanup.
  The adapter now uses native wait/context completion as well as events, keeps
  only observed message IDs and the request binding while waiting, and captures
  only the new completed visible reply. Runtime admission validates that binding
  against the same task/Brain/device and applies both old and current policy.
  The stricter OpenCode test passed in 7.60 seconds; diagnostics were removed.
  A complete native regression including older bridge/runner cases is underway.
- The complete regression exposed two further races: concurrent first inbox
  initialization could read a partial profile, and OpenCode shutdown could lose
  session metadata before final capture. Inbox initialization is now serialized;
  OpenCode uses observed request metadata and waits for the durable local capture
  acknowledgment. All 25 agent unit tests passed, including eight concurrent
  cold inbox opens. The next complete run passed the four older native bridge/
  runner tests and all three installed-plugin tests (including three successive
  OpenCode launches), but three subsequent lifecycle/recovery checks failed.
  `.cache/plugin-native-full-regression-2.log` is intermediate failed evidence;
  combined-run diagnosis remains open. No final acceptance is inferred from it.
- The subsequent diagnostic run identified the lifecycle failures as fixture
  interference: earlier compatibility proofs leave the legacy capture plugin in
  the shared default host home. The plugin correctly refuses duplicate capture.
  Lifecycle fixtures now select their own host configuration and explicitly test
  conflict refusal, unchanged legacy configuration, retained queue bindings and
  recovery after removing only that fixture's conflict.
  [Claude's configuration reference](https://code.claude.com/docs/en/mcp-quickstart#find-your-configuration-on-disk)
  and Context7 confirmed project-local MCP entries in `.claude.json` and the
  `CLAUDE_CONFIG_DIR` replacement location. Inspection now checks the matching
  project entry, honors that override, and reads linked host configuration without
  changing it. Unrelated project entries do not block the current workspace.
- `.cache/plugin-native-full-regression-4.log` passed all four prior native
  compatibility tests and all eight plugin tests (74.74 seconds for the plugin
  group). This includes the explicit legacy-conflict/recovery checks and all
  three consecutive OpenCode prompt/reply launches. Storage review then extended
  checks to linked parent directories and existing non-private state; the first
  workspace test caught a relative-path compatibility issue in the new inbox
  initialization lock. The lock now resolves the already-created owned directory;
  the storage checks remain strict. Final workspace/package validation is pending.
- Workspace Rust tests and strict workspace/all-target clippy then passed.
  The macOS package includes the optimized runtime and Enola 0.4.19. The first
  macOS installed-host attempt exposed a fixture-relative path; the next exposed
  a real migration false positive from scanning beyond the checkout into an
  overridden home configuration. The migration scan now stops at the nearest
  checkout (or the current directory without a checkout), with the selected
  user configuration checked separately. Codex's official local loader source
  confirms the project-layer boundary. Failed owned macOS fixture databases,
  credentials and artifacts were removed after verifying no analytical/erasure
  obligations. Current macOS acceptance remains under test.
- The corrected release package at
  `.cache/plugin-dist/darwin-arm64-final-20261002-b` passed actual macOS Codex
  0.159.3 and OpenCode 2.0.21 ordinary launches. Codex completed in 9.43 seconds;
  OpenCode completed three launches in 21.08 seconds. Both used the installed
  bundled executable and OS credential store, supplied cited context before the
  model call, executed scoped MCP, retained prompt/reply sources, and published
  an owned Git checkout through the sibling Enola executable. No extractor
  override, Rust invocation by the plugin, personal host configuration change or
  additional Keychain approval was needed. The installed runtime performed
  disconnect and the harness cleaned its disposable database and owned files.
  Claude's native compatibility evidence is Linux 2.1.270; Claude is not installed
  on this Mac. Logs: `.cache/plugin-macos-codex-3.log` and
  `.cache/plugin-macos-opencode.log`.
- The package guide and three integration runbooks now separate normal plugin
  setup, optional execution, advanced direct HTTP and legacy queue migration.
  Their transition notices remain until full acceptance and deployment. The UI's
  OpenCode entry now names the native `index.mjs` entry point explicitly.

## Final verification — 2026-10-02

The final Linux native regression (`.cache/plugin-native-final.log`) passed all
four older bridge/runner compatibility cases and all eight plugin cases (80.45
seconds for the plugin group). These are the current private-storage and migration
boundary changes, including ordinary Codex/Claude/OpenCode prompt/reply capture,
three OpenCode launches, automatic cited recall, canonical learning after session
end, compaction hook retrieval, child/scope/delayed-event attribution, reader-only
recall, legacy collision refusal/recovery, original-destination offline delivery,
hard eight-second deadline and optional runner execution/disablement.

Workspace Rust tests passed 46 cases; 177 external-fixture tests were explicitly
ignored there and assessed separately where required. The final agent test run
passed all 25 cases. Strict workspace/all-target clippy passed. The broad platform
run passed 138 of 144 cases initially: two failed because the SFTP fixture was not
selected, and four timed out before their controlled pause/receipt. With the owned
SFTP fixture and without concurrent suites, both recovery cases, all four graph
recovery cases and the MCP protocol-failure restart case passed. The original red
run remains in `.cache/plugin-platform-final.log`; focused results are in
`.cache/plugin-recovery-mirror-rerun.log`, `.cache/plugin-recovery-erasure-rerun.log`,
`.cache/plugin-graph-recovery-rerun.log` and `.cache/plugin-mcp-restart-rerun.log`.
No timeout was relaxed and no production graph/executor behavior was changed.
The script now checks the SFTP prerequisite before running and builds its native
MCP fixture. The six failed disposable databases were removed after ownership,
zero analytics attempts/transactions and graph absence were verified; the owned
SFTP endpoint and generated keys were removed after its successful proofs.

Affected browser checks passed: coding-agent setup 2 cases (one optional live
Context7 case skipped), capture 1 case and MCP 3 cases. Capture's stale assertion
was updated to the delivered Agents link. Typecheck/design/build passed; Vite's
existing bundle-size advisory remains. Six owner-login screenshots at
1280×800, 1440×900 and 1920×1080 were visually inspected, covering the top and
bottom of setup with no horizontal overflow. Evidence lives in
`.cache/ui/plugin-setup-{1280,1440,1920}-{top,bottom}.png` and the three
`.cache/plugin-ui-*-final*.log` files. Small-screen and additional keyboard polish
are optional under the accepted desktop decision.

## Deployment and limits

The release package is `.cache/plugin-dist/darwin-arm64-final-20261002-c`.
macOS Codex/OpenCode installed-host proof and Linux Claude proof are detailed above.
Personal host configuration has not been replaced: installations used owned
homes, credentials, workspaces and disposable servers. Host trust and initial
account authorization remain user setup steps. Native compaction triggering,
paid-model quality/capacity benchmarks, Windows compatibility, public marketplace
publication and OAuth are not inferred from these tests. Compaction retrieval was
proved at the documented adapter boundary using the actual runtime and server.

The final Brain deletion API suite passed both authority/pre-write and full-flow
consumer-denial/fence cases (`.cache/plugin-brain-deletion-final.log`).

`./scripts/stack.sh up --build` completed the existing root Compose upgrade from
029 to `030_plugin_session_memory`. API and worker share image
`0e2a6edcffba362da7c67c176d738a385f485c94808ba54228e73234bf7ba6a0` and
`http://127.0.0.1:8787/health/ready` returns ready. The private pre-upgrade database
backup, before/after inventories and verification report are under
`.cache/plugin-deploy-20261002/`. All original 16 Brains, 26 sources, 27 source
versions, 29 claims, 37 claim revisions and Brain grant rows are preserved.
Source/version/claim/revision/grant rows match exactly. Brain rows match except
`graph_scanned_at` and `graph_discovery_cursor`, which the existing background
worker advances; no content or access change was excluded from the comparison.
Volumes, secrets and the independent deletion journal were retained. The old
backup must not supersede later deletion-journal entries during recovery.

A fresh owner browser login against the deployed installation opened the new
Agents setup at 1280×800, 1440×900 and 1920×1080, with no horizontal overflow or
page errors. The three screenshots under `.cache/plugin-deploy-20261002/` were
visually inspected; the test signed out without changing any Brain or credential.
The six fixture screenshots above additionally cover the lower runner/advanced
options. Governance validation and diff hygiene are part of the final closeout.
No release, commit or push occurred.

## Final packaging compatibility check

The closing docs audit found that the plugin forwarded `workspace refresh/list`
but omitted the companion's read-only `workspace discover` entry point. It now
calls the same bounded discovery helper before loading any connection or OS-store
credential. The release package was rebuilt as `darwin-arm64-final-20261002-c`.
An actual bundled CLI check against an owned Git fixture passed without configured
plugin state, left workspace bytes unchanged and rejected extra arguments. Its
report is `.cache/plugin-offline-discovery-final.json`; all 25 agent tests and
strict agent/all-target clippy passed after the fix. README and workspace/publication
guides now describe the normal packaged workflow. The server/UI image is unchanged
because this addition affects only the bundled plugin CLI.

The final `-c` package then passed fresh installed macOS Codex and OpenCode
checks, including automatic cited context, real scoped MCP, prompt/reply capture
and sibling Enola publication. Codex passed in 9.35 seconds; OpenCode's three
launches and publication passed in 74.27 seconds. Logs are
`.cache/plugin-macos-codex-final-c.log` and
`.cache/plugin-macos-opencode-final-c.log`. Both fixtures disconnected and removed
their owned databases, credentials and files. Final governance lint/all 32 tests,
formatting and diff checks pass. The normal deployed installation remains ready.
