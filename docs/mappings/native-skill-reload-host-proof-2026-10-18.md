# Native skill registration, reload and lifetime evidence

Observed: 2026-10-18
Confidence: official/installed interfaces and observed local native-host proof; normal deployment not tested

## Sources and Method

This successor study follows the user's explicit native install/reload/lifetime
request. The [earlier research/proof](managed-skills-host-consumption-2026-10-18.md)
remains the first slice's evidence, not proof of native consumption.

- [Official Codex skill behavior](https://developers.openai.com/codex/skills/)
  and [app-server interface](https://learn.chatgpt.com/docs/app-server).
- [Claude Code skill lifecycle](https://code.claude.com/docs/en/skills).
- [OpenCode skills](https://opencode.ai/v2/docs/skills) and
  [native plugin SkillContext](https://opencode.ai/v2/docs/build/plugins).
- Context7 `/websites/opencode_ai_v2` confirmed `ctx.skill.transform` with
  add/update/remove and `ctx.skill.reload()` for external transform-state changes.
- Installed CLI reads: `/opt/homebrew/bin/codex --version` returned 0.162.1;
  `/opt/homebrew/bin/opencode --version` returned 2.0.22. Initial `claude --version`
  failed because no Claude executable was on PATH.
- The parent then installed Claude only under ignored
  `.cache/native-skill-hosts/claude/`; its direct darwin-arm64 binary's `--version`
  independently returned 2.1.302. No global installation/configuration changed.
- Codex generated interface artifacts live in ignored
  `.cache/native-skills-research/codex-schema/`. Reads of `SkillsListParams.ts`,
  `SkillsExtraRootsSetParams.ts` and `SkillsChangedNotification.ts` establish
  installed schema, not an actual host refresh/activation result.

## Observations

Codex 0.162.1's generated list parameters contain `cwds` and `forceReload`; its
extra-roots setter is process-scoped. A watched-change notification invalidates
old metadata. Current online examples mention additional per-CWD fields absent
from this installed schema, so the adapter must not assume they work. Dedicated
native skill inputs identify name/path for activation; a catalog read alone
cannot prove its body reached a conversation.

Claude's documented existing-root watcher and new-root reload distinction makes
startup synchronization significant. Inspection of installed 2.1.302 exposed a
`reload_skills` control request and SessionStart `reloadSkills: true` hook output.
The parent's initial official web search found no public documentation for those
two exact fields: retain the installed version/schema and actual control/hook
proof rather than generalizing them to older binaries. The actual startup/control
proof below now confirms this version. Public documentation additionally states
that a root absent at startup remains unwatched after `/reload-skills`; subsequent
changes there require repeating reload or a fresh session with the root present.

OpenCode's official native plugin API supports skill transforms and reload.
Its catalog is location-oriented and discovery includes `.agents/skills`,
`.claude/skills` and `.opencode/skills`. Host-specific locks therefore cannot
prevent conflicting managed scopes in shared compatibility roots. The reviewer
identified this cross-host case before implementation; the accepted design uses
a common workspace-wide scope ledger plus host-specific owned exports.

## Translation and Limits

The [accepted contract](../contracts/mcp-native-skill-reload.md) and
[successor pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-native-skill-reload.md) require
real native registration/activation proof, per-task temporary references, explicit
persistent intent and unchanged-owned cleanup. Instruction bundles become native
skill wrappers while the original AGENTS resource and existing local instruction
hierarchy remain intact.

Workspace exports are visible to other local programs. The shared lease prevents
competing product writes; it is not confidentiality or a promise to retract
conversation context. Native entrypoints require a current scoped canonical
fetch before applying their cached body/references. Placed, discovered, loaded,
updated, removed and reload/restart-required states need different evidence.

## Runtime Acceptance Ledger

Actual installed-host fixtures passed on macOS 27.0.1 arm64. These are the isolated
processes started by the fixture, with separate homes/config and synthetic
loopback model providers. They do not attach to the user's already-running
coding session or normal Recollect installation.

| Host/version | Observed native proof |
| --- | --- |
| Codex 0.162.1 | Native catalog and explicit name/path activation delivered v1 then v2 body/reference markers in the same process/thread. Temporary two-task references, retained instruction wrapper/source, persistent uninstall and preserved existing local files passed. |
| Claude Code 2.1.302 | Native catalog/activation delivered v1/v2 and referenced text in the same process/session through actual `reload_skills` control calls. A separate startup fixture created a previously absent root during SessionStart; `reloadSkills: true` produced native discovery, activation and reference reads. Two-task temporary cleanup, retained instruction/source and uninstall/local-file preservation passed. This does not claim a watcher for roots absent at startup. |
| OpenCode 2.0.22 | The production `createNativeSkillReload` helper used real `ctx.skill.transform` and `ctx.skill.reload`. Same-process/session v1/v2 body/reference and instruction activation passed, along with temporary/retained cleanup and preserved local files. A later run reconciled a compatibility-root duplicate introduced after the first snapshot; actual reload counts advanced 1 through 7. Removed native activation returned HTTP 404. |

Local receipts are ignored fixture artifacts:

- `.cache/native-skills-proof/98ecb198-e5a5-4235-8521-2329c7aab69b/results.json`:
  all three hosts passed, zero paid provider calls, with body/reference markers,
  two live temporary task references, retained instructions and explicit uninstall.
- `.cache/native-skills-proof/b6bf8c87-064d-461e-8cde-15183c8bdbcc/results.json`:
  the additional OpenCode production-transform/late compatibility-duplicate run
  passed. Its `opencode/host-events.json` records reload counters `[1,2,3,4,5,6,7]`.

Additional real shipping-entrypoint proof passed in
`.cache/native-skills-proof/offline-cli-4c441af1-d596-4abc-b04c-9ea0ac2fd151/results.json`.
The freshly built `target/debug/recollect-plugin` completed eight status/leases/
reload/temporary-uninstall/retained-uninstall actions with an absent credential
slot and unavailable loopback server. The global default Brain differed from the
workspace's recorded Brain; receipt-bound local cleanup still selected the correct
workspace and preserved its existing instructions. No network request was needed.
This is local maintenance proof, not permission for a fresh offline install.

The repository reproduction is `python3 scripts/test-native-skills.py --host all`.
It builds `crates/agent/examples/native_skill_fixture.rs`; `--host codex`,
`claude_code` or `opencode` narrows the run. Explicit executable overrides use
`RECOLLECT_CODEX_BIN`, `RECOLLECT_CLAUDE_BIN` and `RECOLLECT_OPENCODE_BIN`.
Fixtures use no personal credentials/configuration and no paid inference.

The final full agent library suite passed 94 tests, including canonical-cache,
native engine and CLI/lifecycle cases. Nine JavaScript adapter tests passed.
Coverage includes multiple task references, retained pins, cross-host scope
conflicts, offline end/recovery, current operation checks, unsafe/modified/foreign
files, receipts, complete-snapshot failures and bounded materialization.

Final review identified a delayed old-scope fetch applying after a live task
transition. The completed fix registers the server-observed scope generation
before network work and checks it under the common workspace lock for export,
release and recovery/catalogue metadata publication. Deterministic regressions
cover delayed old fetch/denial, stale metadata writes and interrupted transition
cleanup in both recovery phases. The final reviewer reported no blockers.

The updated bundled `recollect-memory` skill passed frontmatter/native-command
vocabulary checks and exact-name/path catalog matching in actual Codex 0.162.1 `skills/list` discovery in an isolated
process with no provider call. Its separate documentation-check receipt is
`.cache/native-skills-proof/doc-skill-discovery-315c181a-e8c0-4d6b-88a4-d8b15a43e797/result.json`;
this check deliberately makes no body-activation claim. Public operator guide
build/check passed: 28 canonical guides, 31 HTML routes, 1,579 local references
and anchors, 113 static files. Post-edit `./scripts/validate.sh` passed governance
lint, 35 governance tests and 10 support-baseline tests; scoped diff check passed.

Automatic native network work is bounded to 2.4 seconds, the hook to 7.5 seconds
and its process watchdog to eight seconds. Private task/snapshot state binds
workspace/device/task/scope; the final 8 KiB escaped hook envelope retains the
native pointer and supported Claude startup action after other context producers
merge. OpenCode scope-busy snapshots preserve an active peer's registry; an
unavailable transition without a peer clears the changing task's stale registry.
Final policy/generation regressions and review passed. A later global
`./scripts/validate.sh` attempt failed on the separately authored concurrent
`docs/contracts/retrieval-attributed-feedback.md`, which lacked its Explicit
Deferrals section. That unrelated work was not edited during native delivery;
the earlier 35+10 pass remains distinct from that failed attempt. After archive,
the public guide
rebuilt and passed all 31 routes, 1,579 local references/anchors and 113 static
files. Scoped governance passed through the unchanged validator's filled-pack,
metadata, governing-authority and link checks, plus exact active/archive/epic
index comparisons. Fresh standalone governance/support suites passed 35+10 tests;
Rust format, JavaScript syntax and scoped diff checks passed. The final whole
validator then stopped at the unrelated feedback contract, which was preserved.

Following the user's explicit authorization on 2026-10-18, a fresh inspection
found the feedback contract's Explicit Deferrals section already present. The
full `./scripts/validate.sh` rerun passed governance lint, all 35 governance tests,
the frozen support baseline and all 10 support-baseline tests. The temporary
repository-wide validation blocker is resolved; no duplicate section or product
behavior change was needed.

## Verification Limits and Follow-up

These receipts prove native transport/activation and local ownership/lifetime
behavior. They explicitly set `canonical_service_connectivity_tested=false`;
canonical HTTP/MCP grants, disable/history/revocation/deletion were independently
proved in the [first slice](managed-skills-host-consumption-2026-10-18.md).
Synthetic providers verify exact body/reference delivery, not semantic model
quality or guaranteed model obedience to the generated canonical-fetch instruction.
Workspace exports remain visible to local readers and cannot retract existing
conversation content. No installed user plugin was upgraded, normal server
migration applied, release made, commit/push performed or service deployed.

The [archived successor pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-native-skill-reload.md)
records the completed local delivery. The first shipped pack remains unchanged.
