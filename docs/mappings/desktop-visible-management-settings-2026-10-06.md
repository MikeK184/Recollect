# Visible management settings and concise navigation

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- Accepted [desktop follow-up](../contracts/desktop-experience.md),
  [managed experience](../contracts/memory-managed-experience.md) and
  [semantic recovery](../contracts/retrieval-semantic.md) amendments govern the
  seven user-marked corrections. Original ten-point delivery remains in its
  [dated mapping](desktop-browser-management-2026-10-05.md).
- Source inspection of `App.tsx`, `AgentsRoster.tsx`, `McpPanel.tsx`,
  `toolDescription.ts`, `ModelsPanel.tsx`, `CapturePanel.tsx`,
  `PrivacyOverview.tsx` and shared management/provider CSS.
- Production `npm run build` passed type, design and Vite checks. The two pure
  `tool-description.spec.ts` tests passed; they do not launch a browser. Existing
  browser regression expectations were updated, but that browser suite was not
  executed for this follow-up. Actual UI proof below uses CUA only.
- Final `./scripts/stack.sh up --build` succeeded; API, worker, PostgreSQL and
  Neo4j healthy, readiness returned `{"ready":true}`. Build log:
  `.cache/visible-management-stack-build.log`.
- CUA exercised the existing owner installation at `http://127.0.0.1:8787` in
  the user's 1384 × 1473 viewport. No viewport override, disposable database
  fixture, external metadata refresh or tool execution was used.
- Independent UI agent reviewed source and actual saved screenshots. A
  protected-inline-code list boundary defect and preview inconsistency were
  found and fixed before the final build and captures.

## Observations

| User point | Current local evidence |
| --- | --- |
| Bottom Team menu item | Ordinary icon/name navigation row directly above the account row; selected on Team. Existing installation-owner authority retained. [Team](../../output/browser-review-2026-10-06/team.jpg). |
| Redundant roster link | Brain roster has zero `agent-roster-footer` elements; header and global My agents navigation remain. [Agents](../../output/browser-review-2026-10-06/agents.jpg). |
| Extra connection metadata | Read-only preview omits More configuration, redundant description and empty credentials text. Existing Target/Scope/Runs on Edit fields remain and Cancel restores. [Connection](../../output/browser-review-2026-10-06/connection.jpg). |
| Tool description formatting | Real cached Context7 full description renders seven paragraphs, three bullet lists and one numbered list, with no metadata refresh. Preview uses the same formatted display text. [Tools](../../output/browser-review-2026-10-06/tools.jpg). JSON schema has 18 source lines and 45 highlighting spans through the existing bundled renderer. [JSON](../../output/browser-review-2026-10-06/tools-json.jpg). |
| Visible AI processing and logo | Privacy reminder removed; Automatic processing checkboxes visible, not in a disclosure. Provider icon/name vertical centre difference measured 0px. [AI](../../output/browser-review-2026-10-06/ai.jpg). |
| No routine coverage review | Normal AI settings mounts no Search coverage card or Review items prompt. Optional Diagnostics remains. Existing recovery source audit below establishes the bounded automatic-or-idle behavior. |
| Visible Privacy controls | Capture limits and Activity & backup windows are visible sections; zero details elements inside Privacy management. Empty exclusions omitted from read mode, configured arrays still visible/enforced. Capture Edit permits 1–64 KiB and optional exclusions. Draft changed to 64 KiB plus a disposable exclusion, then Cancel restored 32 KiB and the empty read view. [Privacy](../../output/browser-review-2026-10-06/privacy.jpg), [Capture edit](../../output/browser-review-2026-10-06/privacy-edit.jpg). |

Read-only SQL before and after the cancelled capture draft confirmed unchanged
policy revision `4493b7bf-dfca-4a25-8fc9-7495e0b3560b`, 32768 bytes, zero excluded
tools and zero excluded content rules. No real policy, retention, grant, token,
connection or model selection was saved for this evidence. Existing inventory
remains 16 Brains, 35 devices, two connections and one tool group.

### Existing automatic recovery audit

The second implementation agent audited existing source without changing it:

- `crates/server/src/worker.rs` runs autonomous/semantic maintenance repeatedly.
- `semantic/queue.rs::retry_automatic` permits at most two new recorded attempts
  after 5/30 minutes for confirmed rate-limited/unavailable failures, under current
  profile, policy and input eligibility. These replace recorded attempts; they
  do not silently replay uncertain requests.
- Pre-request budget, concurrency and policy blocks are reconsidered after one
  minute under current authority. Missing provider configuration pauses admission.
- Permanent, uncertain, exhausted, erased and obsolete work stays idle safely;
  independent work and exact/lexical retrieval continue. No mandatory human
  coverage review or new paid retry authority is introduced.
- Narrow existing boundary: credentials disappearing after a job is queued can
  produce `model_credentials_missing` classified as failed with no request.
  Restoring credentials does not automatically revisit that entry. It remains
  idle, within the user's explicit recovery-or-ignore choice.

Relevant existing proof sources inspected, not rerun: platform semantic retry,
budget, lost-response/independent-work/erasure tests; semantic recovery policy
change tests; model-catalogue blocked rebuild authorization tests. Publication
fences and `migrations/017_semantic_retrieval.sql` preserve policy/input authority.
This is current source evidence, not a fresh live provider or timed retry probe.

## Translation and Limits

Legacy descriptions containing line breaks retain their original bytes. For old
flattened descriptions, render-only formatting restores explicit colon/bullet,
numbered-list and capitalized section boundaries outside protected code spans.
Pure checks preserve source words/order, original line breaks/fenced JSON and
single/double-backtick code, including bullet names separated by inline spans.
Stored metadata is unchanged; original lost whitespace cannot be recovered exactly.
The existing safe Markdown renderer handles source line breaks and highlighted
code; no Prism download, new dependency or external call was needed.

Privacy editing remains revisioned and authorized through Edit/Save/Cancel;
exclusion arrays are preserved in whole-policy drafts. Hiding an empty read-view
row does not remove its optional editor or alter redaction/capture behavior.
The original backup-adapter limitation stays truthful in the visible section.

No backend schema or retry implementation changed in this follow-up. Fresh
OpenCode direct-token launch acceptance remains separately open in its own pack.
Version N/A; all work local and uncommitted, with no push or external release.

## Follow-up

No remaining acceptance work for these seven corrections. Repository governance,
whitespace and CodeGraph checks are recorded in the reconciled pack. Optional
diagnostic inspection remains available without a routine review requirement.
