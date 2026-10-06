# General, bounded Privacy and limited demo scenario

Observed: 2026-10-06
Confidence: verified

## Sources and Method

User requested images first, then selected the General and inline environment
layouts. Built-in image generation produced three light concepts before runtime
scenario writes. [Exact prompts and concepts](../../output/imagegen/2026-10-06-settings-refinement/prompts.md)
are retained; illustrative values are not runtime evidence.

Authority: [desktop amendment](../contracts/desktop-experience.md#bounded-privacy-rows-follow-up--2026-10-06),
[team access](../contracts/platform-team-access.md),
[device pairing](../contracts/platform-device-pairing.md),
[MCP catalogue](../contracts/mcp-catalogue-and-profiles.md),
[direct MCP auth](../contracts/mcp-plugin-direct-auth.md), and the
[General execution pack](../roadmap/execution/archive/desktop-general-inline-environments.md).

Source checks: production type/design/Vite build, four pure Privacy draft/save
cases, updated browser regression discovery (`--list` only), repository governance,
whitespace and synchronized CodeGraph. Independent agent inspected source and
final owner/reader/focused-row pixels. Browser proof used CUA on the running local
installation, not fixture-writing browser tests against a real Brain.

Full `./scripts/stack.sh up --build` stopped at its 12 GiB free-space guard before
any service change. No caches or data were deleted. A 3,600,982-byte frontend-only
image layer used the exact existing backend image
`sha256:7de277c25678cd9773a4fd7710aaad3c2e80169212093937183cfd491d1f5df7`,
then normal `./scripts/stack.sh up` recreated API/worker and applied the canonical
migration step. Logs: `.cache/general-privacy-web-image.log` and
`.cache/general-privacy-stack-start.log`. Readiness returned `{"ready":true}`.
This is new frontend delivery over the existing backend, not a fresh backend build.

## Observations

### Settings

- General uses Brain details and environments in the main column, with a compact
  side card for status/archive and existing permanent deletion. Name, description
  and icon edit inline; creation and identity/icon partial-save semantics remain.
- Environment Add and Rename appear inline with named regions and Save/Cancel.
  Existing list stays visible while adding. Current read errors, admin/archive
  gates, create idempotency and fresh-read description preservation remain.
- Historical admin metadata editing on archived Brains is preserved; environment
  editing closes on archive/role/read loss. Reader sees real values without edit,
  add, rename, archive or deletion actions. No grants or API semantics changed.
- At 2504 × 1314, General measured 1200 px and Privacy 1120 px without horizontal
  overflow. Privacy titles/tabs align with the bounded form; rows have fine 1 px
  separators and a sage focused-row background/inset accent.
- Manual Excluded tools/content controls are absent in both Privacy read/edit
  views. Full cloned draft retains existing exclusion arrays; automatic secret
  redaction and saved backend rules remain unchanged. No real policy save was
  performed. Brain details, Add, Rename and Privacy editors were all cancelled.

Screenshots: [General](../../output/demo-review-2026-10-06/owner-general.png),
[inline Add](../../output/demo-review-2026-10-06/owner-environment-add.png),
[focused Privacy](../../output/demo-review-2026-10-06/privacy-focused.png),
[wide Privacy](../../output/demo-review-2026-10-06/owner-privacy-wide.png), and
[reader General](../../output/demo-review-2026-10-06/demo-general.png).

### Limited account and actual agent

The local installation contained no `demo` account before this scenario. Owner
invitation/enrollment created non-owner account
`1725db3a-c74f-4573-b065-64c6cc79b77c`; only reader access was granted to local
SWEG — test (`5c054930-d266-4c18-a42b-942729f942aa`). Its password satisfies the
existing eight-byte minimum; no password is recorded in this evidence.

Pairing created `Demo MCP · SWEG`, device
`d7eaafe6-beab-4b8d-89dd-b36bdea46843`. The generic Direct MCP host is truthfully
unreported; this is not a native Codex/Claude/OpenCode session claim. Bearer is
kept only in an ignored 0600 local file. Canonical task/tool-operation bindings
preceded its real managed Exa call; no SQL activity/device fabrication was used.

| Demo action | Actual result |
| --- | --- |
| List Brains | Exactly SWEG — test, reader |
| My agents | Exactly its own one agent; real observed use |
| Read/use Demo web research | Allowed; two Exa tools discoverable |
| Manage/share that group | Both 403 |
| Change Brain name or capture policy with valid input | Both 403 |
| Team, global definitions or Brain Access read | 403 |
| Another existing Brain read | 404 |
| Brain metadata/environment/archive/delete UI | Mutation controls absent |
| Tool-group UI | Use green; Manage/Share red; no edit/person-management control |

The use-only profile read correctly omits other people's grants/effective
identities. Owner sees both names with adjacent permissions; reader sees its own
effective rights. [Owner permissions](../../output/demo-review-2026-10-06/owner-tool-access.png),
[demo permissions](../../output/demo-review-2026-10-06/demo-tool-access.png),
[demo Brains](../../output/demo-review-2026-10-06/demo-brains.png), and
[own agent](../../output/demo-review-2026-10-06/demo-my-agents.png).
Owner session was restored; the user's existing Graph tab was preserved and the
temporary proof tab/viewport cleaned up.

### Added no-auth MCP

[Exa's official MCP page](https://exa.ai/mcp) verifies its anonymous endpoint
`https://mcp.exa.ai/mcp`. Canonical inspection rejected its explicit Draft 7 input
schemas, because approval requires bounded 2020-12. Independent audit found only
assertions with identical validation semantics; offline original/reviewed/runtime
validators agreed over 18 positive/negative cases per tool. The reviewed manifest
changes only two root dialect declarations, preserves every assertion and uses
the standard descriptor projection. Product schema validation was not weakened.
Raw metadata, candidate and repeatable Rust proof remain in ignored `.cache`.

Canonical approval created definition `demo-exa-anonymous-20261006`, connection
`b694e0b8-8c47-4e3e-846e-a9848d3cd7fe`, and tool group `Demo web research`
(`d034fa97-ec4b-4f3a-b168-435f8037ef19`). Explicit demo Use-only and owner Use
grants were added only to this new group. Existing Context7 groups/grants unchanged.

Actual demo-agent call `d0ec9c5f-420f-47eb-8fc3-685f262a093e` to `web_search_exa`
succeeded with `connector_response` and a public `rust-lang.org` result, without
a tool-error flag. It used a public query/objective and no external credential.
`capture_disposition=knowledge_write_required` correctly prevented a reader from
writing the result into Brain memory. No paid provider operation was submitted.
Non-secret identifiers and bounded outcomes are retained in
[the resource ledger](../../output/demo-review-2026-10-06/resources.json).

One added MCP fulfills the requested 1–2 range. DeepWiki and Microsoft Learn were
also probed at their official no-auth endpoints; neither was created or reported
as connected. Their inspection returned `provider_initialization_failed`.
Read-only investigation isolated current SDK/protocol error handling: DeepWiki's
uncorrelated discovery error and Microsoft Learn's SSE discovery error prevent
automatic legacy fallback. A future compatibility slice needs separate authority
and actual runtime proof; importing metadata alone would not fix startup.

## Translation and Limits

Final read-only SQL inventory: 16 Brains, 2 accounts, 36 devices, 2 approved
definitions, 3 connections across Brains and 2 tool groups. Changes are the newly
requested demo account/grant/device/task/operation and one Exa definition/group/
connection, not deletion or replacement of existing data. Demo resources remain
for user inspection. No customer-infrastructure action, real settings save,
archive/delete, credential overwrite or unrelated host acceptance was performed.

Owner/reader functional browser proof and API denials establish this local demo
scenario, not every authorization permutation or native host integration. Existing
regression selectors parse; their fixture-mutating browser cases were not run.
Backup scheduling is still not proven by this UI work. Separate OpenCode native
token-host acceptance remains open after its earlier service startup timeout.

## Follow-up

General and bounded Privacy delivery accepted locally after independent review.
No remaining work in these two slices. DeepWiki/Microsoft protocol compatibility
is recorded as a separate limitation. Version N/A; work remains uncommitted.
