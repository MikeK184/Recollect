# Ten-point browser management delivery

Observed: 2026-10-05
Confidence: verified for the local implementation and recorded checks; observed-once for account availability and rendered pixels

## Sources and Method

The user approved all ten browser comments and the follow-up Connections-style
tool-group cards, with independent Use/Manage/Share icons beside each name.
The accepted [desktop amendment](../contracts/desktop-experience.md),
[direct authentication amendment](../contracts/mcp-plugin-direct-auth.md),
[catalogue](../contracts/mcp-catalogue-and-profiles.md),
[runtime](../contracts/mcp-runtime-and-credentials.md), and
[model catalogue contract](../contracts/brain-model-catalogue-and-selection.md)
govern the implementation. The [original investigation](../research/browser-management-review-2026-10-05.md)
records the before measurements and external interface research.

Parent and delegated implementation agents reviewed source, canonical API
authority and isolated PostgreSQL/provider fixtures. The independent visual
agent reviewed current source and saved actual CUA browser captures. Browser
interactions used CUA on the rebuilt installation and an owned, disposable
database at a distinct localhost origin. No automated Playwright browser suite
was executed in this task. Updated regression sources remain for normal CI.

## Observations

| Point | Delivered behavior | Recorded proof |
| --- | --- | --- |
| 1. Connectors scale | Shared 248px sidebar, 14px navigation and 40px page heading; compact cards and shared controls | Same-viewport computed styles match Brains, My agents, Brain Agents and Connections at 1897 × 1314 |
| 2. Diagnostics | Persistent top-right icon for authorized operators, including healthy state; Team remains in account controls | Actual healthy API/PostgreSQL/Neo4j diagnostics opened from Brains; Team visible in account menu |
| 3. Agents scale | Both agent views use the shared shell and controls | Current Brain/global routes inspected; sole memory-read button corrected to shared 36px height |
| 4. Token setup | Masked copy-ready Codex TOML, Claude JSON and OpenCode v2 JSON with literal Authorization; visible Access tokens; unique issuance | Synthetic native-format parse passes; UI-issued disposable config copied privately; Codex native CLI recognizes it; real initialize, tools/list, workspace.list and memory.recall succeed with its header |
| 5. Agent activity | Selected-agent inline inspector and date-grouped compact timeline; per-event evidence/details expand | Existing Codex activity selected; Questions event expands its evidence and bounded technical details |
| 6. Automatic contexts | Private-context/local-folder controls hidden in normal navigation; old contexts URL normalizes to Agents | Legacy URL resolved to canonical Agents without context/folder controls; native scope APIs retained |
| 7. Connection editing | Existing inspector becomes editable; stable Save/Cancel and sections; guarded selection while editing | Owned fixture Save/Cancel, fixed 390px inspector, disabled competing actions, retained revision-conflict draft, Reload then Save verified |
| 8. Code formatting | Bundled lowlight for JSON/TOML/YAML/shell and safe Markdown; discovery preserves newlines | Four fenced languages highlighted, exact JSON copy including newline, zero active script/tracking-image elements; inner code border/padding both 0px |
| 9. Tool groups | Sage-header Connections-style cards with MCP and People columns; adjacent green/red effective icons and labelled Direct edit controls | Synthetic reader/writer/owner rights, second MCP persistence, configuration-plus-grant partial saves, repeated retry receipts and Cancel verified at 1280 × 720 |
| 10. AI settings | Inline permissions, supported account-available models, dated prices, explicit compatible embedding rebuild; defaults applied on creation | Actual account catalogue loaded five supported text IDs and two embedding IDs; local draft costs/dimensions and Save and rebuild warning verified, then Cancel; synthetic checkbox Save persists after reload; backend generation/selection tests pass |

The first ownership review found three real editor defects: effective rights
could show draft values, connection actions could change selection during an
edit, and partial-save receipts could disappear after retry. These were fixed.
The final visual review found no blockers. Its two minor formatting observations
were corrected and the final code/Agents captures reviewed again successfully.

### Focused validation

- Frontend API generation reports 184 unique operations. Final `npm run build`
  passes TypeScript, shared design checks and the production build.
- Server and MCP runtime Clippy with warnings denied passed. Ten server library
  units, including four icon rejection/normalization cases, pass. The runtime
  discovery test preserves Markdown/newlines while retaining the existing
  2,000-character/control-character limits.
- Isolated PostgreSQL MCP tests
  `mcp_independent_profile_powers_admin_no_use_and_foreign_rls` and
  `mcp_schema_approval_cold_discovery_pagination_and_reconfiguration` pass.
  The own-account/Brain roster tests and pairing/revocation/worker-authority
  test also pass.
- Model catalogue units (2) and catalogue integration cases (3) pass, including
  admin/CSRF, bounded errors/cache, reviewed availability/prices, revisions,
  immutable profiles, dimensional DB guards, rebuild replay, in-flight old
  output suppression, erasure and lexical access. The initial model suite
  passed 53/54; its large-scope semantic admission query hit the unchanged
  two-second bound. An identity-only admission preflight repairs this without
  changing ranking projection or relaxing the deadline. All 19 semantic cases
  pass on the follow-up, including that case.
- A no-browser native-configuration test passes exact synthetic TOML/JSON
  parsing and unique issuance. The actual copied disposable literal header
  succeeds through ordinary HTTP MCP. The empty test Brain returns zero recall
  items: this proves authenticated recall execution, not content retrieval.
- An already-normalized synthetic PNG was approved through the canonical API;
  its local cached image loads at 256px natural width in both list and inspector.
  The absent-icon definition retains its generic fallback. This is isolated
  cache/render proof, not a claim that a live external server publishes icons.
- Final `./scripts/validate.sh` passes governance lint and all 32 checker tests;
  `git diff --check` passes. CodeGraph synchronized 24 changed files and 622
  nodes after the final source correction and reports its index up to date.

### Local deployment and preservation

`./scripts/stack.sh up --build` completed for the final source. API, worker,
PostgreSQL and Neo4j report healthy, and `/health/ready` returns `ready: true`.
Migration 034 is applied after 033. Existing installation inventory remains
16 Brains, 35 devices, two connections and one tool group.

The local build's 12GiB free-space guard initially stopped a rebuild after
development compiler artifacts filled the disk. Only exact task-generated
incremental/object artifacts were removed; the final guarded build then passed.
No source, application data, unrelated cache or Docker inventory was removed.

All test-only grant/token/configuration changes used the owned fixture. Its
server/database, disposable token and private copied configuration files were
removed through the fixture's privacy/graph cleanup. The temporary browser tab
was closed and copied credential cleared from the clipboard. Installation
policy, model pair, grants, tokens and connection configuration were not changed
for visual testing. The account model-list refresh sends installation
authentication to the configured OpenAI account API; it sends no Brain corpus
and makes no paid model inference request.

### Actual captures

- [Tool access, actual installation](../../output/browser-review-2026-10-05/implementation-tool-access.jpg)
- [Populated green/red permission card, synthetic fixture](../../output/browser-review-2026-10-05/implementation-tool-access-fixture.jpg)
- [Inline Direct permission controls](../../output/browser-review-2026-10-05/implementation-tool-access-edit.jpg)
- [Partial-save retry receipts](../../output/browser-review-2026-10-05/implementation-tool-access-partial-save.jpg)
- [Retained connection conflict](../../output/browser-review-2026-10-05/implementation-connection-edit-conflict.jpg)
- [Safe highlighted code](../../output/browser-review-2026-10-05/implementation-code-formatting.jpg)
- [Cached synthetic icon](../../output/browser-review-2026-10-05/implementation-cached-icon.jpg)
- [Compact agent activity](../../output/browser-review-2026-10-05/implementation-agents.jpg)
- [Masked direct configuration](../../output/browser-review-2026-10-05/implementation-direct-config-masked.jpg)
- [AI model/rebuild draft, subsequently cancelled](../../output/browser-review-2026-10-05/implementation-ai-edit.jpg)

## Translation and Limits

All ten feature changes are implemented locally and uncommitted. Five packs have
completed their recorded acceptance. The
[direct-token pack](../roadmap/execution/active/mcp-copy-ready-agent-config.md)
keeps its final fresh native-host launch acceptance open: the installed OpenCode
CLI timed out waiting for its background service to start. Configuration parsing
and successful HTTP protocol calls are recorded separately and do not establish
a successful OpenCode session. No unidentified existing host service was stopped.
Fresh Claude launch acceptance was not exercised in this run.

Account availability was observed once on the running installation; provider
execution is independently fixture-tested and remains governed by standing
policy. Prices are dated standard metadata, not an account invoice or live quote.
Embedding changes require rebuilding and may remain blocked by policy or missing
credentials. No real installed Brain was rebuilt to obtain screenshots.

Optional MCP icons require initialized metadata and the approved bounded formats
and locations. Many servers publish no eligible icon; the generic fallback is
intentional. Old already-flattened discovery descriptions cannot be reconstructed
without an explicit refresh. Browser sources authored here have not been run as
an automated browser suite. No commit, push, version bump or external release was
requested; version N/A.

## Follow-up

Complete the retained direct-token native-host acceptance using a disposable
credential once the installed OpenCode background service starts successfully.
Do not expand real grants, copy credentials into tracked files or conflate native
configuration recognition with a live host connection. No other accepted UI
implementation remains. LongMemEval, marketplace publication and OAuth remain
separate work.
