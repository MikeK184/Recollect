# Desktop and Brain deletion continuation

Observed: 2026-10-01
Confidence: observed-once

## Sources and Method

Read-only reconciliation of the handover against Git base `6a85f6a`, followed
by the user's authorization to finish four slices and use a second agent for
visual review. Governing sources are ADRs 0016/0017 and the four scoped execution packs.
`desktop-ask-primary` remains held. Local logs are under
`.cache/desktop-continuation-20261001/`; screenshots are also under `.cache/ui/`.

Dependency checks used Context7's TanStack Query and Cytoscape documentation
and the installed TypeScript interfaces for cancellation, cache events, fitting
and node positions. The concentric layout now includes label dimensions and fits
inside measured overlay insets; its degree grouping keeps small graphs readable. These developer-tool calls do not prove Brain-managed MCP.

## Observations

- The handover understated existing wiring implementation: the base commit
  already moved incoming setup to Agents and resolved legacy Connections tabs.
- Knowledge now opens one canonical lineage drawer directly. Record history and
  actions remain explicit secondary views. Exact source versions, supporting
  evidence, derived memories and bounded neighbourhoods use canonical reads.
  Graph controls retain their selection/path actions inside the shared drawer.
- Browser regression exposed cancellation of an inspector read resetting the
  whole graph. Reset events now apply only to the graph's own read keys; mutation
  and access invalidation remain. Independent owner review confirmed label readability, no overlaps or control
  occlusion at 1280/1440/1920, nested Escape/focus, exact-revision cross-view
  selection and history routing. Graph canvas geometry was 479px versus 174px
  combined chrome at 1440x900 in the disposable browser fixture.
- Assurance uses existing authorized metadata feeds. Learning figures describe
  the latest completed run, not Brain lifetime totals. Captured-event totals are
  passed through from the Brain-wide server count. Private tool output is not
  retained in the band's cache. Five focused assurance cases passed, including
  current blockers versus historical failures.
- Brain deletion needed a lossless decimal-string closure counter across JSON:
  its 60-bit value could not safely round-trip through a JavaScript number.
  The companion also incorrectly expected local cleanup counts in the server
  fence response. Both fixes have real HTTP coverage. Four focused Rust
  deletion cases and the disposable browser deletion case passed, including
  wrong-name refusal, stale-preview rejection, disappearance and cleanup.
- Sources/evidence, retention, graph analytics, combined graph, claims,
  lineage, investigation, full desktop navigation and optional review browser
  checks passed, as did final graph, MCP-runtime and list-hygiene reruns.
  The graph stress fixture rendered 500 nodes/2,000 edges and selected an entity
  in 1,195ms; invented fixture IDs correctly fail canonical claim reads. Expiry
  clears the canvas and disposes the renderer. This is a synthetic rendering
  observation, not a production capacity claim.
- The isolated installed-host rerun passed Codex 0.154.0 and Claude Code
  2.1.270 native discovery, workspace tools, fresh scope context and capture
  publication. All four `test-mcp-hosts.sh` cases passed, including native
  scoped managed calls and real local/private stdio and HTTP runners.
- The earlier simplification removed optional review/correction controls even
  though the accepted desktop and review contracts preserve them. Restored the
  existing canonical handlers behind the explicit inspector action. Browser
  review passed correction, conflict resolution, lost-response replay and stale
  decisions; autonomous learning remains the default without an approval queue.
- Workspace Rust tests passed 39 cases; Clippy and formatting passed. Platform
  integration initially passed 141 cases; two recovery cases required the SFTP
  fixture and one native provider failed initialization. All three passed focused
  reruns (the two recovery tests used the owned real SFTP fixture). OIDC and
  installed-host cases remain separately selected by their dedicated harnesses.

## Bounded measurements

Twenty warm local samples per operation recorded canonical graph reads at
64.1ms p95, pointer-to-entity-detail at 134.6ms p95, and exact/lexical recall at
43.8ms p95. The graph fixture has four entities and two edges. Twenty navigation
samples per route measured click-to-heading, not fully settled backend data:
Memory 34.1ms p95, Sources 34.6ms, Graph 35.9ms and Repositories 35.2ms.
API requests observed before that heading boundary were 2/1/4/1 on the first
visit respectively, then 0/0/1/0 on each of the remaining 19 visits. Full arrays
and timing boundaries are retained in `.cache/ui/graph-repeated-measurements.json`
and `.cache/ui/desktop-repeated-measurements.json`. These observations do not
establish production capacity or improvement over a pre-redesign baseline.

## Translation and Limits

The current work is local and uncommitted, with no release or push. Disposable
UI databases on port 8788 test the current code. The native process on 8787 and
its older container worker are separate runtime evidence; the current backend
fixes have not been deployed there. Existing Brains were not deleted for proof.

Owner-login visual review uses the native installation with rebuilt UI assets.
The owner session was restored after correcting a browser-helper readiness race.
Build 8 passed Graph and cross-view review at 1280, 1440 and 1920; the subsequent
optional correction dialog passed the same three widths on build 9, including
unsaved-edit cancellation. Build 10 passed the 1440×900 owner spot-check: historical 8080 → Latest
knowledge 9090 remains in the same history dialog, with `detail=record` retained
and stale revision/time parameters removed. Review used existing records without
mutating the owner’s Brain.

Ordinary UI tests now generate a disposable owner password and remove paid
provider credentials unless explicitly opted in. Provider-backed behavior uses
separate controlled platform fixtures; real-provider/installed/identity and
benchmark opt-ins are reported as skipped when not exercised.

ADR 0017's Wiring row says "Team access", while the desktop contract retains
global owner-only Team and the implementation places Settings in Wiring.
This wording discrepancy remains recorded without moving Team or widening
permissions. The existing default Ask route predates this continuation; holding
`desktop-ask-primary` does not reverse that earlier route change.

## Current blockers, not historical alarms

The user questioned the vague yellow warning in the live demo and chose
current blockers only. Read-only inspection found a `source.learn` job from
2026-09-14 with `provider_timeout`, while current processing reports up to date.
That historical event did not establish a present blocker. The band now escalates
only current provider prerequisites, canonical failed processing, unresolved
uncertain tool outcomes or erasure errors. Failed-job/tool history, pending
cleanup and capture reports remain in a separate Activity section. Warnings name
the problem, omit unrelated metrics and link directly to the diagnostic owner
when there is one blocker. No historical records or live jobs were modified.
The second agent confirmed the live demo is quiet and its old failure remains
inspectable on build 11 at 1440×900. All five browser cases passed; the second
agent also approved current-blocker and history-only fixture screenshots.

## Display and interaction priority

The user's 2026-10-01 decision targets normal laptop screens and desktop monitors
up to about 32 inches; the [desktop contract](../contracts/desktop-experience.md#display-and-interaction-priority--2026-10-01)
is the authority. Additional small-screen and keyboard/focus polish are optional.
The repeated graph inspection test now uses ordinary pointer selection; no new
keyboard behavior or shortcuts were introduced. Existing browser focus checks
that already pass remain useful coverage, without expanding that work. A flaky
MCP nested-dialog focus-return assertion was reduced to verifying that the
original evidence control is visible; capture and recovery assertions remain.
Graph pointer selection now names the exact Amber relationship instead of
selecting an arbitrary first relationship.

## Validation and closeout

All four scoped packs are locally delivered and archived. The final browser
matrix covers all 37 current spec files: **65 cases passed and 7 opt-in cases
skipped**. Evidence is `browser-final-matrix.tsv` under the local log directory.
The skipped cases are Atlas lifecycle, autonomous live-provider acceptance,
installed UI proof, live model calls, public benchmark, recovery UI and team OIDC.
They are not new runtime proof. The separate real SFTP recovery and installed-host
Rust fixtures did run successfully.

The initial combined browser rerun passed Ask (5), capture (1) and recall-graph
(1); its other failures were resolved in isolated spec fixtures. Final browser
acceptance is the aggregate of those passes and isolated reruns, not a claim
that the earlier combined command was green. Retired `list-density.spec.ts` and
`graph-debug.spec.ts` duplicated or contradicted maintained coverage; list-hygiene,
knowledge-surface, graph and graph-chrome retain the meaningful checks.

Workspace Rust: 39 passed, with integration opt-ins ignored by that command.
Platform integration: 141 initial passes, followed by successful focused reruns
of the two SFTP recovery cases and one native-provider initialization case, plus
all four installed/native MCP cases. The remaining OIDC opt-in was not rerun.
Focused Brain deletion: four passed. Clippy with warnings denied, Rust formatting,
web design/typecheck/build and the final docs/whitespace checks passed. Vite still
reports its existing large-chunk advisory; this is not a build failure.

The disposable harness now separates owner credentials, preserves failed-browser
evidence, and allows bounded native startup/drain time for the observed macOS
pre-main loading delays; successful queue draining is still required. Owned failed
platform fixtures were reconciled through privacy/analytics cleanup before their
exact databases were dropped; unrelated old fixtures were preserved.

The external Context7 call passed through the host-facing Recollect MCP transport:
anonymous discovery plus actual `resolve-library-id` for React returned useful
library results, rather than a quota error. The three direct-MCP browser cases
passed; all four installed/native host cases also passed. Earlier combined
runs triggered normal login throttling and shared-fixture assumptions, so final
reruns use one fixture per spec. The external probe first lacked the required `context_query`; this was
a test-client schema error, corrected before the successful rerun.

The owning epic maps and active/archive indexes are reconciled. Product platform
is complete locally; MCP coordination and operational readiness retain their
other open slices. The six earlier desktop acceptance packs, held Ask slice,
OpenCode project setup and plugin packaging are outside these four closeouts.
Final backend deployment to the existing installation remains separate.
