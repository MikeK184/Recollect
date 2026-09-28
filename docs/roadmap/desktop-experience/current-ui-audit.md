# Current UI audit

Status: proposed
Observed: 2026-09-26
Purpose: supporting evidence for the desktop-experience plan.

## Method and limits

Three read-only reviews inspected current routes/components, accepted contracts,
Rust agent/model interfaces, local Atlas/Cognee references and primary external
documentation. CodeGraph status/sync was checked before structural navigation.
Supplied screenshots and the preceding live-browser inspection establish layout,
not every runtime state. No application actions or tests were executed by these
reviews. Test source establishes intended coverage, not a new passing test run.

The current application has four routes in
[main.tsx](../../../web/src/main.tsx) (22–40). One
[BrainDetail](../../../web/src/App.tsx) (856–901) mounts every major panel.
There is no generated-answer consumer in this screen.

## Classification

- H: frequent/occasional human use.
- S: setup or administration.
- A: normal agent/background responsibility.
- D: diagnostics, recovery or optional intervention.

These describe UI prominence, not new authorization roles. Most controls are
backed by real APIs. Moving controls is presentation work; changing defaults,
authority, models, storage, automatic requests or aggregates may be product work.

## Complete function relocation matrix

| Current capability | Class | Proposed home | Boundary |
| --- | --- | --- | --- |
| Brain list/create/open, archived state | H | Global Brains | Existing behavior; aggregate counts need response verification |
| Edit Brain name/description | S | Settings / General | Existing admin authority |
| Manage Brain members/roles | S | Settings / Access | Preserve effective grants and owner protection |
| Archive/reopen | S | Settings / General | Preserve history; separate from Erase |
| Import text/file/reference source | H/S | Sources / Add source | Do not claim arbitrary file/connector support |
| Source content, version history, provenance, availability | H | Sources / detail | Exact versions and unavailable bytes remain visible |
| Source edit/organize/association changes | H/S | Source detail | New attributable versions; no external Git writeback |
| Collections, areas, environments | H/S | Source Manage views and page scope chips | One association model; grouping removal is not erasure |
| Allow document content retention | S | Settings / Retention; import summary | Preserve current data/policy until explicit save |
| Reprocess / explicit Learn source | A/D | Source detail overflow | Normal learning remains automatic under policy |
| Exact/text/semantic/graph recall | H/A | Ask / Search evidence | Existing retrieval remains available separately |
| Recall modes, result limit, context budget, time, manifests, channel detail | A/D | Advanced search | Keep criteria visible and eligibility unchanged |
| Matches / Disagreements / Sources | H | Evidence-search result tabs | Same bounded response; no hidden fan-out or model call |
| Copy attributed context | H/A | Result action | Copy only valid returned context |
| Generated answer and follow-up | H | Ask default | NEW consumer, model purpose, API and citation/retention contract |
| Claim/decision/procedure browse | H | Memory | Existing lists; proposed text search needs new authorized server filtering before pagination |
| Manual memory creation/revision | H/S | Memory secondary action | Not a prerequisite for autonomous learning |
| History/review/correct/reject/withdraw/resolve conflict | H/D | Memory detail | Same canonical commands; no mandatory review inbox |
| Handover generate/status/refresh/retry/contributors | H/A/D | Memory / Handovers | Existing model policy and provenance |
| Graph kind/selection/scope | H/A | Graph compact toolbar | Exact repository/manifest selection unchanged |
| Graph canvas/node/edge/source inspection | H | Graph + right inspector | Main content first; no fake edges |
| Path endpoints/direction/hop limits | A/D | Contextual Find path and Advanced | Same qualified traversal |
| Graph analytics, reports, coverage | H/A/D | Graph / Insights | Explicit bounded background work |
| Graph rebuild, epochs, generations, unresolved links | D | Status popover; Activity / Processing | Repair controls remain reachable |
| Repository catalogue/origin aliases | H/S | Repositories and detail | Add origin registers identity alias, not clone |
| Snapshot files/facts/coverage/contributors/insights/receipt | H/D | Repository detail tabs | Exact committed input and coverage |
| Snapshot reprocess/erase | D/S | Snapshot detail | Existing storage and command rules |
| Environment manifests and revision history | H/S | Repositories / Environments | Records versions; does not deploy |
| Your checkouts and local observations | A/D | Repository/Agents own workspace inspector | Account/device-specific local paths |
| Connect coding agent | S | Agents / Setup | Incoming memory MCP, not outgoing tool server |
| New/child task, scope change, close | A/D | Agents / Task scopes/detail | Private scope bookkeeping, not to-do management |
| Operation binding and scope history | A/D | Session context inspector | Immutable operation scope |
| Refresh catalogue button | D | Last updated/error retry | Browser GET refetch; does not discover local repositories |
| Capture host setup | S | Agents / Setup | Supported hooks/host versions only |
| Capture event/content permission | S | Settings canonical editor, Agents link | Capture != provider transmission |
| Captured sessions/events and sources | H/A/D | Agents / Sessions | Published evidence differs from private task metadata |
| Capture delivery gaps/failures/filtered/skipped | D | Activity with source/session detail | Do not claim complete transcript coverage |
| Approved connector catalogue/Add connection | S | Connections wizard | Operator approval of definition remains prerequisite |
| Approve/install arbitrary MCP server in browser | S | Separate future design | NOT currently implemented |
| Profiles/connections/environment and Use/Manage/Share | S | Connections / Profiles | Knowledge access does not confer execution |
| Cached tool schemas | S/D | Profile/connection detail | Cached availability is not observed connectivity |
| Manual tool call, output, cancel, reconcile, resolve, release | A/D | Canonical call inspector via Connections/Activity | Test label cannot make an effectful call safe |
| Private runner registration and placement | S/D | Connections / Runners | Device identity stays global |
| Installed model/purpose/content/budget/transmission policy | S | Settings / AI & automation | Preserve explicit permission; provider/model identity currently deployment-selected/read-only |
| Autonomous learning configuration | S | Same settings section | Existing default operating model, not a new review requirement |
| Model connection checks | S/D | Settings explicit action | Presence of credentials != successful call |
| Semantic search config/status/rebuild | S/D | Settings + Activity | Repair action secondary |
| Learning output/model request/token history | D | Activity / Model usage and learning filter | Actual usage; no invented success or mandatory approval |
| Retention periods by content class | S | Settings / Retention | Source expiry affects what can be verified/rebuilt |
| Source/memory/snapshot/manifest erasure preview | H/S | Contextual owner action | Preserve preview, explicit authority, deletion fences |
| Erasure pending cleanup/status | D | Activity / Data removal | Backup/control limits remain visible |
| Processing/jobs/retry/cancel | D | Activity / Processing | Preserve uncertain/nonretryable effects |
| Brain audit | S/D | Activity / Changes | Admin-only data stays admin-only |
| Installation health/runtime diagnostics | D | Global status and owner drawer | Healthy config is not runtime proof |
| Team invitation/OIDC enrollment/enable/disable/reset/revoke | S | Global Team | Account sign-in is distinct from Brain access |
| Account activity | S/D | Team / account detail | Owner authority |
| Pair/approve/decline/revoke companion | S | Global Devices | Own device identity and current user authority |
| Sign-in/invite acceptance/OIDC recovery/expiry/sign-out | H/S | Auth flows/account menu | Theme applies; not main submenu destinations |

## High-impact findings

### Manual controls obscure existing automation

[ADR 0006](../../adr/0006-autonomous-memory.md) and
[autonomous maintenance](../../contracts/memory-autonomous-maintenance.md)
already define learning, reconciliation and retirement without per-record human
approval. The redesign should align with this accepted intent. Optional
inspection/correction remains a product requirement.

### Graph is currently configured before it is seen

[GraphPanel](../../../web/src/GraphPanel.tsx) (415–664, 733–775) puts selection,
scope, rebuild and path forms before
[GraphExplorer](../../../web/src/GraphExplorer.tsx). Make bounded content the
primary surface; preserve filters, analytics, exact snapshots and diagnostics in
contextual controls. A list/table alternative is required for keyboard access.

### Tasks and refresh labels are easy to misunderstand

[WorkspacePanel](../../../web/src/WorkspacePanel.tsx) (110–135) polls/refetches
server metadata. Native discovery is a different companion operation.
[Workspace scope](../../contracts/evidence-workspace-scope.md) makes tasks
account-private scope containers, even from other Brain admins. A new Agents
page must not introduce cross-user task visibility.

### Simplification must retain meaningful failure states

Current source supports missing bytes, partial extraction, stale scope,
permission removal, unknown tool outcomes, budgets, erasure and expiry. A green
dot cannot replace those distinctions. Hide stale protected payload on failed
authorization/refetch, while preserving safe user-entered form values.

### Mounting every panel also mounts irrelevant polling

Examples: [JobsPanel](../../../web/src/JobsPanel.tsx) polls around 1.5 seconds,
[ModelsPanel](../../../web/src/ModelsPanel.tsx) learning around 2 seconds,
[GraphPanel](../../../web/src/GraphPanel.tsx) around 3 seconds, and workspace
around 4 seconds. These are source observations, not a measured performance
result. Active-route mounting can remove unrelated requests, but required
visible-content expiry/revocation monitoring must remain.

## Key source locations

| Area | Source |
| --- | --- |
| Source/view/import/detail controls | [EvidencePanel](../../../web/src/EvidencePanel.tsx), 164, 438, 719, 942 |
| Search controls/results | [RecallPanel](../../../web/src/RecallPanel.tsx), 395–792 |
| Memory kind/forms/list | [MemoryForms](../../../web/src/MemoryForms.tsx), 25–30; [ClaimsPanel](../../../web/src/ClaimsPanel.tsx), 1220–1399 |
| Workspace, repositories/checkouts/tasks | [WorkspacePanel](../../../web/src/WorkspacePanel.tsx), 148–350 |
| Snapshot/manifests | [PublicationPanel](../../../web/src/PublicationPanel.tsx) |
| MCP catalogue/profiles/runners | [McpPanel](../../../web/src/McpPanel.tsx), 155–355 |
| Model config/learning/usage | [ModelsPanel](../../../web/src/ModelsPanel.tsx), 772–919 |
| Capture | [CapturePanel](../../../web/src/CapturePanel.tsx), 431–535 |
| Retention and erasure | [RetentionPanel](../../../web/src/RetentionPanel.tsx), 496–594 |
| Team and devices | [TeamPanel](../../../web/src/TeamPanel.tsx), 103–264; [DevicesPanel](../../../web/src/DevicesPanel.tsx), 47–203 |

## Acceptance focus

Existing behavioral browser coverage includes
[investigation](../../../web/tests/investigation.spec.ts),
[graph](../../../web/tests/graph.spec.ts), source/workspace/MCP/capture/memory
flows, and meaningful failures. This audit did not rerun it. Screenshot capture
exists, but no snapshot-comparison or axe assertion was found in the inspected
test source.

Add focused route/deep-link/back-forward/Brain-switch/role-loss tests, preserve
epoch/expiry/correction behavior, and prove no navigation or tab switch triggers
a model request or effectful tool call. Test actual submitted scope and exact
revision navigation through shared inspectors. Check desktop widths, keyboard,
focus, contrast, long IDs/origins, and a small set of visual baselines. Preserve
positive useful flows alongside forbidden-data tests.

## Mockup boundary

Images use fictional names, content, status, counts, dates and credentials
references. These are not current customer data or proof of API support.
Map every summary/aggregate to a verified response or explicitly scoped new read
API before implementation. Generated badges do not certify deployment or
review; a configured connection is not a successful call.
