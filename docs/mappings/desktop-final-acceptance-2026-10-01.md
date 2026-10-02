# Final desktop deployment and host acceptance

Observed: 2026-10-01
Confidence: observed-once

## Sources and method

The user resumed Ask-primary, plugin/OpenCode work and the six older desktop
acceptance packs, and authorized deployment to the existing local installation.
This record follows the [four-slice continuation](desktop-continuation-2026-10-01.md)
and preserves the [September 26 implementation evidence](desktop-experience-implementation-2026-09-26.md)
as dated history. Logs and screenshots are in the ignored, owner-only
`.cache/final-closeout-20261001/` directory. No credentials belong in this record.

## Deployment and preserved state

The prior native API and older Compose worker were replaced by the root Compose
API and worker built from the final code. Both run image `64d06daac505` with the
updated web assets and backend. Migration completed successfully and readiness
passed. Existing storage, environment and credentials were preserved.

Pre/post deployment inventory is identical: 16 Brain identities, 26 sources,
27 source versions, 29 claims and 37 claim revisions, plus the recorded grant,
model-policy, retention and capture digests. A private pre-upgrade database backup
was retained. It must not be restored over later erasures without the canonical
deletion journal. Disposable answer-proof Brains were deleted through the product
and their cleanup completed; existing user Brains were not used for mutations.

## Desktop acceptance

- The completed continuation matrix covers all 37 then-current Playwright spec
  files: 65 passes and seven explicit opt-in skips. It includes corrected Evidence,
  Publication, Claims, Retention, Graph, Analytics, Investigation, MCP, Operations,
  Workspace and Capture journeys. The detailed matrix remains in the earlier
  evidence directory; this closeout does not relabel skipped tests as passes.
- Two separately enabled real-provider browser cases now pass: model settings,
  transmission and budget behavior; autonomous learning of port 8080, revision to
  9090 and refreshed handover with no human reviewer. Earlier failures were stale
  assertions against collapsed advanced controls; the tests now open those controls.
- A deployed actual-provider question returned a qualified answer and two exact
  citations. Opening a citation resolved its source version. An authorized source
  update and UI correction changed the claim to 9090 with the new exact support;
  subsequent explicit Search returned the corrected assertion. The disposable
  Brain was deleted and cleanup confirmed. Navigation and Search selection issued
  no provider call. The final successful journey made one answer request; earlier
  bounded attempts also made requests and are not included in that one-call count.
- This journey exposed a real lineage bug: the correction dialog lacked its scope
  catalogue, leaving Save disabled. The lineage consumer now loads the existing
  workspace catalogue on demand, offers pending/error/retry states and passes it
  to the canonical editor. A focused browser regression proves a changed assertion
  with a new exact source span, rather than merely changing rationale.
- Five Ask regressions pass, including default Brain redirect, disabled fallback,
  temporary answer/citation behavior and a 1280x800 visible composer. The empty
  composer now precedes suggestions; completed conversation layout is unchanged.
  A combined Ask/lineage run passed ten cases, then hit the normal ten-logins/minute
  limit on its eleventh. The focused correction rerun passed independently.
- Activity acceptance combines owner diagnostics/ordinary-member denial, private
  output canaries, role-loss clearing, empty/partial/error feeds, late Brain-switch
  responses and canonical exception links. Actual MCP-runtime tests cover permitted
  output, uncertain-call recovery, cancellation, expiry and Use revocation. The
  production workspace integration test supplies foreign-account task/path canaries;
  Activity does not broaden those endpoint permissions.
- Independent owner visual review approved all twelve destinations and Ask at
  1280x800, 1440x900 and 1920x1080, including the disabled Search fallback. The exact
  citation screenshot was inspected; the corrected-value assertion comes from the
  runtime check, since its screenshot cuts off before the value. Small graph captions
  at 1280 remain usable and are clearer at larger widths.

The user's [display and interaction priority](../contracts/desktop-experience.md#display-and-interaction-priority--2026-10-01)
governs: normal laptops and larger monitors up to about 32 inches; additional
small-screen and keyboard/focus polish is optional. The assurance band continues
to escalate current blockers only, with historical failures retained in Activity.
The ADR 0017 Wiring wording discrepancy remains recorded in the continuation
mapping; Team authority was not moved or widened.

## Performance and carried proof

The continuation's twenty-sample local measurements remain applicable to the
unchanged graph/recall implementation: graph reads 64.1ms p95, pointer-to-detail
134.6ms and exact/lexical recall 43.8ms. Heading-visible navigation measured
34.1/34.6/35.9/35.2ms for Memory/Sources/Graph/Repositories with first-visit request
counts 2/1/4/1, then 0/0/1/0. These are bounded local observations, not settled-data
latency or production capacity. The synthetic 500-node/2,000-edge graph rendered
and selected an entity in 1,195ms. The real answer journey records actual answer
state and provider accounting, not a new answer-stage p95 study.

Earlier current-code Rust proof passed 39 workspace tests and the platform suite
with three documented focused reruns for SFTP/native initialization. Four actual
installed-host/native cases passed in the owned Linux harness (Codex 0.154.0,
Claude Code 2.1.270, synthetic model transport), and Recollect-managed Context7
completed a useful React lookup. Earlier OIDC, installed/recovery browser and
provider-quality evidence remains dated, rather than claimed as fresh reruns.

## OpenCode developer setup

Installed OpenCode 2.0.21 discovers all six repository roles and both shared
documentation skills from the root and nested `web` directory. All six role
bodies match the Codex definitions exactly. An actual model session observed root
instructions and completed Context7 `resolve-library-id` for React and
chrome-devtools `list_pages`. Configuration discovery and successful tool calls
are recorded separately in `opencode-proof.json`.

The user's custom default provider points to an unavailable tunnel. The proof
used a dedicated loopback OpenCode server and a process-only OpenAI credential
with a one-off model selection; personal defaults and saved provider accounts
were not changed. Initial implicit-location CLI output was empty; explicit root
and nested locations became available after the host finished loading. A fresh
session and completed discovery are necessary before judging parity.

Dependency evidence used Context7's official OpenCode v2 documentation and the
installed host, plus the official [v2 OpenAPI schema](https://opencode.ai/v2/openapi.json),
[agents](https://opencode.ai/v2/docs/agents),
[MCP servers](https://opencode.ai/v2/docs/mcp-servers) and
[CLI](https://opencode.ai/v2/docs/cli). These developer tools are separate from
Brain-managed tools and do not imply OpenCode session capture.

## Codex plugin and native credential proof

The installed local generated `recollect-capture` 0.1.0 plugin is enabled, and its
memory skill is present. OpenCode stdio configuration is generated without secrets.
Fresh native proof uses only a disposable Brain and a distinct paired device
profile. The existing default device profile and customer workspaces are unchanged.

Live proof exposed three macOS FFI defects in the Keychain access-list helper:
`SecTrustedApplicationCreateFromPath` takes a C string, `SecAccessCreate` takes
three arguments, and the retained access reference must not be released twice.
The implementation now matches the installed Apple SDK headers. A meaningful
native unit test exercises access-list creation without reading credentials;
all 19 agent tests and agent all-target Clippy pass. The credential metadata
contains exactly the companion, bridge and runner trusted application paths.
The official [Security access APIs](https://developer.apple.com/documentation/security/secaccess)
and [access-control lists](https://developer.apple.com/documentation/security/access-control-lists)
provide the external interface context.

Installed Codex 0.159.3 completed a real model-driven direct-HTTP workflow on the
same disposable Brain: empty scope, one contribution, a separate retrieval
operation, exact recall and provenance inspection. Claim `f660e7ca-1f26-4dac-85c6-3c9f7b4a9f6c`
contains value 9191, proposed review state, declared operational state, current
freshness and unknown validity, supported by source version
`2d918a1e-6bcc-4061-83ab-728a4720ba76` line 1. Invalid task/enum attempts were
rejected before the model corrected its arguments; only one contribution succeeded.
The proof's temporary direct token was revoked afterward. Its bounded synthetic
content went to the selected model; no customer memory was used.

The installed Codex prompt catalog resolves the memory skill to the plugin cache
under `recollect-capture/recollect-capture/0.1.0/skills/recollect-memory/SKILL.md`.
A separate actual Codex session read that installed SKILL.md, opened a fresh task
and successfully recalled/inspected the contributed claim. Both temporary direct
tokens were revoked. A first proof guessed a missing cache directory. Inherited personal host settings
also stalled the code-mode tool runner; a process-only clean invocation restored
normal file/MCP tools without changing personal configuration. These host failures
are distinct from Recollect's valid HTTP tool responses.

The native bridge initially waited in `SecItemCopyMatching` and OpenCode timed
out while OS approval was unresolved. After the user's approval completed, the
readiness probe initialized and listed all 22 tools. Fresh companion, Codex and
OpenCode processes then succeeded without another user step. The actual Codex
session read the installed skill, started a fresh empty-scope task and recalled /
inspected value 9191 with its exact provenance. OpenCode 2.0.21 connected through
the rendered native stdio configuration and its model completed `workspace.list`.
An expired OpenCode location initially returned 404 on reconnect; loading that
location before connecting resolved it without configuration changes.

The actual `capture run` collision preflight refused the fixture's direct-HTTP
`recollect` entry with exit 1, unchanged config and an empty capture evidence
directory. It did not launch a host or capture operation. These results are saved
in `native-host-final.json`, `opencode-bridge-proof.json`, `codex-events-final.jsonl`
and `collision-proof.json`. Existing Keychain entries were not migrated and no
broad OS access-policy change was made. OS approval remains user-controlled;
replacing binary identities may require approval again.

Cleanup completed through canonical product paths: the disposable Brain was
deleted and returned 404, deletion cleanup reached complete, its device was revoked
and the distinct local profile was forgotten. Both temporary HTTP tokens are
revoked. The dedicated OpenCode proof server on 4099 was stopped; the user's other
host sessions were preserved. `inventory-final.json` matches all original 16
Brain IDs and the five recorded inventory counts. API readiness remains true.

## Validation and limits

Web typecheck/design checks and the final production image build passed. Agent
tests and Clippy pass after the FFI repair. Fresh workspace library tests passed 34 cases, with three live-Vault cases
explicitly ignored. Documentation validation and all 32 governance tests pass;
`git diff --check` and `cargo fmt --all -- --check` pass. CodeGraph synced all seven
changed source files.

All nine resumed desktop/developer/plugin packs are closed against the evidence
above. No fixture Brain or active proof credential remains. LongMemEval's paid
answer-level benchmark, published marketplace/OAuth and OpenCode capture hooks
retain their separate decisions/non-goals. No external deployment, release,
commit or push was performed. Version: N/A. Commit: uncommitted.
