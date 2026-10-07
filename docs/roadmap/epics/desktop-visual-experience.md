# Desktop Visual Experience: Animated Modern Desktop UI

Status: complete

## Purpose

Make the desktop product feel alive and modern in the visual language the user
clarified on 2026-10-03: the original light cream/ink/sage theme with per-category
accents, fluid purposeful motion, terminal-style agent output surfaces, an
animated live pipeline view of capture → learning → memory/graph flow, and an
ambient fullscreen memory display. Every existing function, route, data feed and
permission is preserved. The approved control-panel revision adds a bounded
read projection for truthful activity presentation. Desktop-first; no mobile redesign.

## Governing Sources

- [Desktop contract](../../contracts/desktop-experience.md) (amended 2026-10-03: visual language and motion)
- [Design system document](../desktop-experience/design-system.md)
- [claude-mem study, 2026-10-03](../../research/claude-mem-study-2026-10-03.md) (reference for terminal-card, TV and live-feed visual language; not implementation authority)
- [ADR 0014: desktop experience and answers](../../adr/0014-desktop-experience-and-answers.md)

## Dependencies and Boundaries

The motion foundation slice lands first; the pipeline view, terminal surfaces and
ambient mode build on its tokens and primitives. The user-approved control-panel
revision adds one bounded server read projection for exact activity correlations
and permits Motion/React Flow for the approved flat design. Earlier shipped
slices retain their original narrower scope. SSE broadcast remains deferred;
the selected feed explicitly refreshes every two seconds. All surfaces keep
their data, routes, URL state, permissions and failure
behavior; presentation changes never change what a page measures or authorizes.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `desktop-dashboard-refresh-stability` | shipped | contract-backed | pack | Preserve fresh pipeline layout across visibility and metadata refreshes without extending validity |
| `desktop-input-errors-and-page-recovery` | shipped | contract-backed | pack | Clear learning failure reasons, guarded input diagnostics and stale page recovery |
| `desktop-inline-processing-limits` | shipped | contract-backed | small-fix: explicit placement correction to existing controls and scoped CSS; canonical fields, bounds and Save/Cancel unchanged | Move Processing limits into the main AI policy form with a responsive two-column grid |
| `desktop-private-runner-relationships` | shipped | contract-backed | pack | Approved device/connection/tool/group cards with reader-safe binding summaries and independent UI acceptance |
| `desktop-private-runners-spacing` | shipped | contract-backed | small-fix: explicit alignment correction to existing scoped CSS; registration, device binding and authority unchanged | Align Private Runners with Tool access and reduce excess tab-to-toolbar spacing; relationship display remains a proposal |
| `desktop-inline-private-runners` | shipped | contract-backed | pack | Approved compact cards and inline Add/Edit delivered locally with immutable device, authority and revision guards |
| `desktop-private-runners-label-concepts` | shipped | contract-backed | small-fix: explicit tab/copy rename and concept-only images; routes, registration and editor behavior unchanged | Private Runners label plus compact read and inline Add/Edit proposals |
| `desktop-tool-access-header-badges` | shipped | contract-backed | small-fix: explicit badge/alignment correction in one existing header; canonical draft/save behavior unchanged | Separate environment/MCP badges and keep environment editing beside its metadata |
| `desktop-compact-tool-access` | shipped | contract-backed | pack | Approved tighter cards, single permission rows and source popovers; canonical retry and independent laptop/wide acceptance |
| `desktop-readable-mcp-inspection` | shipped | contract-backed | pack | Scope/runner badges, reliable tool-read states, single formatted descriptions/JSON and structured call inspection |
| `desktop-general-inline-environments` | shipped | contract-backed | pack | Selected General layout and stable inline environment add/rename over existing commands |
| `desktop-bounded-privacy-rows` | shipped | contract-backed | small-fix: explicit width/separator correction and removal of two presentation controls; existing draft payloads and commands unchanged | Cap Privacy width, highlight focused rows and hide manual exclusions while preserving saved rules |
| `desktop-unified-privacy-form` | shipped | contract-backed | pack | Single Privacy form with title separators and shared edit actions, preserving separate canonical saves and truthful recovery fields |
| `desktop-visible-management-settings` | shipped | contract-backed | pack | Seven follow-up corrections: bottom Team, concise inspection and visible AI/Privacy controls with optional recovery diagnostics |
| `desktop-consistent-management-scale` | shipped | contract-backed | pack | Shared shell scale, persistent diagnostics and removal of automatic-context navigation |
| `desktop-readable-management-content` | shipped | contract-backed | pack | Compact agent activity and safe highlighted management code/Markdown |
| `desktop-inline-management-editors` | shipped | contract-backed | pack | Stable inline connection/AI editing and Connections-style tool-group cards with adjacent permission icons |
| `desktop-management-vision-fidelity` | shipped | contract-backed | pack | Correct six management screens to the approved vision, with independent rendered comparison |
| `desktop-management-concepts` | shipped | contract-backed | pack | Implement six user-approved management concepts, global connector library and bounded credential/config setup, with independent before/vision/runtime UI review |
| `motion-design-foundation` | shipped | contract-backed | pack | Warm dark token system with per-category accents, motion tokens and shared animation primitives (entrance stagger, pulse, skeleton, page transition) applied to the core shell |
| `live-pipeline-visualization` | shipped | contract-backed | pack | Animated capture → learning → memory/graph pipeline view on the Activity surface with processing/queue indicators, animating state diffs over existing polling feeds |
| `terminal-agent-surface` | shipped | contract-backed | pack | Terminal-card component for agent/tool output in the web UI plus an animated banner and status presentation in the Rust plugin CLI |
| `desktop-light-palette-restoration` | shipped | contract-backed | small-fix: explicit restoration of the existing palette and its presentation adapters; no new behavior or interfaces | Restore cream/ink/sage, preserve motion/pipeline/terminal features, adapt category and ANSI foregrounds for light surfaces |
| `memory-tv-ambient` | shipped | contract-backed | pack | Fullscreen ambient mode cycling recent memories with slow fade/scale transitions, toggled from Activity |
| `desktop-final-polish` | shipped | contract-backed | pack | Compact Graph/Connections controls, explicit setup activity destination, direct Privacy editing and measured lazy presentation loading |
| `desktop-actionable-management` | shipped | contract-backed | pack | Generic connection setup and explicit tests, concise tool access, member list/popups, inline privacy, optional environments and automatic learning presentation |
| `desktop-graph-led-workspace` | shipped | contract-backed | pack | Main 3D/2D Graph, bounded contribution perspective, Explore and simplified rich Ask |
| `desktop-flat-graph-correction` | shipped | contract-backed | small-fix: explicit rejection of 3D with an existing accepted 2D replacement; no new interface or behavior | Remove rejected 3D presentation and reuse the existing 2D renderer; assess Graphify viewer |
| `desktop-graph-interactions` | shipped | contract-backed | pack | Smart labels, truthful local type filters, focused connections and compact evidence summary |
| `desktop-guided-setup-and-controls` | shipped | contract-backed | pack | One-step agent setup, compact contexts, connector choice and automatic privacy |
| `desktop-management-simplification` | shipped | contract-backed | pack | Contributor-first Agents, contextual activity and diagnostics, tool access and clear privacy/AI permissions |
| `desktop-brain-pages` | shipped | contract-backed | pack | Approved Dashboard landing, concise Ask and scalable Knowledge inspection |
| `desktop-brain-identity-and-navigation` | shipped | contract-backed | pack | Focused Brain chooser, uploaded icons, personal grouped Agents and Brain-scoped contributors; access drawer and pairing redirects |
| `desktop-live-control-panel` | shipped | contract-backed | pack | Approved flat light global pages and real source/agent/job pipeline with exact attribution, freshness and failure handling |

The archived motion foundation records its original dark-theme delivery; the
light-palette restoration supersedes that color choice without reopening its
motion acceptance. Ambient TV and final polish are accepted and archived as of
October 4; the closeouts below supersede earlier remaining-work notes. The
subsequent actionable management correction is also accepted and archived.

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `motion-design-foundation` | — |
| `live-pipeline-visualization` | `motion-design-foundation` |
| `terminal-agent-surface` | `motion-design-foundation` |
| `memory-tv-ambient` | `motion-design-foundation` |
| `desktop-live-control-panel` | `motion-design-foundation`, `live-pipeline-visualization` |
| `desktop-brain-identity-and-navigation` | `desktop-live-control-panel` |
| `desktop-brain-pages` | `desktop-brain-identity-and-navigation` |
| `desktop-graph-led-workspace` | `desktop-brain-pages` |
| `desktop-graph-interactions` | `desktop-graph-led-workspace` |
| `desktop-management-simplification` | `desktop-brain-identity-and-navigation`, `desktop-graph-led-workspace` |
| `desktop-guided-setup-and-controls` | `desktop-management-simplification` |
| `desktop-final-polish` | `memory-tv-ambient`, `desktop-guided-setup-and-controls` |
| `desktop-actionable-management` | `desktop-final-polish`, `desktop-guided-setup-and-controls` |
| `desktop-management-concepts` | `desktop-actionable-management` |
| `desktop-management-vision-fidelity` | `desktop-management-concepts` |

## Deferred research backlog (not slices)

From the [claude-mem study](../../research/claude-mem-study-2026-10-03.md),
recorded as candidates for future epics — no packs now: 4-layer
progressive-disclosure recall contract; tiered near-duplicate reconciliation for
the learning worker; raw-evidence tier with stable identity and truncation
markers; hook exit-code discipline in the plugins; observer/extractor prompt
hardening; cost-annotated session-start index; reinforcement-based ranking;
curated-answer corpus layer; sync-on-write blueprint for future multi-device.

## Light palette correction closeout — 2026-10-03

- Planned: Restore the user's original light palette while keeping the new visuals.
- Shipped: Cream/ink/sage tokens, original logo and button colours, light terminal
  chrome, readable category/ANSI foregrounds and corrected CSS variable references;
  local Compose rebuilt with browser proof. See [evidence](../../mappings/desktop-light-palette-2026-10-03.md).
- Not shipped: TV completion and overall epic closeout; research backlog deferred.
- New blockers: No palette blocker. TV cached-content/expiry handling remains
  implementation work in its active pack.
- Docs updated: Contract, design system, this epic/index, TV pack, evidence/index
  and `CONTINUE_HERE.md` now retain the light-theme decision.
- Validation: Web build/typecheck/design checks, 32 governance tests, whitespace,
  CodeGraph sync, local readiness, live browser at 1440/1920, reduced motion and
  terminal component/contrast proof passed.
- Version: N/A; no release policy or version bump.
- Commit: Uncommitted; no commit, push or release.

## Live control panel closeout — 2026-10-03

- Planned: Implement the approved flat light proposal with real current/last-observed attributed processing.
- Shipped: Exact source/job/run projection and shared Brains/Activity pipeline, global page layouts and drawers, credential/health semantics, finite observed-change motion and reduced-motion/freshness handling. Locally deployed with preserved inventory; [dated evidence](../../mappings/desktop-live-control-panel-2026-10-03.md) and [archived pack](../execution/archive/desktop-live-control-panel.md).
- Not shipped: TV acceptance, overall epic closeout, SSE and the deferred research backlog.
- New blockers: None for the control panel; TV's existing failure/expiry issue remains separately tracked.
- Docs updated: Contract, pack, epic/index, mappings/index and handoff.
- Validation: Real-worker API proof, browser source/state/failure/resize/permission checks, independent review, build/typecheck/design, clippy, dependency audit, CodeGraph and governance checks passed. Live browser read 30 actual inputs across four contributor contexts; no page errors.
- Version: N/A; commit: uncommitted, no push or release.

The archived initial pipeline pack is historical. Its aggregate animation is
superseded by exact source correlations; graph projection is displayed independently.

## Brain identity and navigation closeout — 2026-10-03

- Planned: Apply browser feedback, support custom Brain artwork, and implement the selected personal grouped Agents list.
- Shipped: Brains opens directly without activity; PNG/SVG/ICO icons; own agents globally grouped by used Brain; safe observed contributors across users within a Brain; account access/history/pairing in Agents Manage access. Local stack rebuilt and inventory preserved. [Evidence](../../mappings/desktop-brain-identity-2026-10-03.md), [archived pack](../execution/archive/desktop-brain-identity-and-navigation.md).
- Not shipped: Ambient TV acceptance and overall epic closeout; research backlog remains deferred.
- New blockers: None for this slice.
- Docs updated: Contracts, pack, epic/index, evidence/index and handoff.
- Validation: Real database/HTTP authorization and deletion/replay, browser conversion/retry/navigation, clippy, build/design/typecheck, independent review, CodeGraph and governance passed. Deployed browser reports no errors.
- Version: N/A; commit: uncommitted. No push or external release.

This correction supersedes the global Brains pipeline and owner-wide global
Agents scope from the preceding slice. Native `/devices?code` links retain their
meaning through the Agents drawer compatibility redirect.

## Brain pages closeout — 2026-10-03

The approved light-theme Dashboard and Knowledge redesign is delivered locally
and deployed to the existing stack. Dashboard is the default, Ask keeps its
composer visible, Memory shows values and retains search across kinds, Sources
uses one inspector, Graph leads with the canvas, and Repositories is searched
and paged on the server. Exact versions, qualifications and retained actions
remain available. [Delivery and validation](../execution/archive/desktop-brain-pages.md).

Independent UI review, desktop layout/accessibility, existing Ask tests, focused
database checks, web build, clippy and governance pass. No new dependency,
release or commit. Ambient TV acceptance and overall epic closeout remain open;
the nine research ideas stay deferred.

## Graph-led workspace closeout — 2026-10-03

The 3D renderer delivery below is historical, superseded by the flat graph
correction. Other delivered behavior remains current.

The main destinations are Dashboard, Ask, Graph and Explore. Wiring is secondary;
Memory kinds are filters. Practical bounded 3D/2D, exact recorded contributions,
formatted evidence and qualified automatic retrieval are implemented and live
in the existing local stack. [Delivery and dated evidence](../execution/archive/desktop-graph-led-workspace.md)
records focused checks, independent UI review, real camera/fallback/failure and
scope proof, performance limits and unchanged 16-Brain inventory. Work remains
uncommitted. Ambient TV acceptance and overall epic closeout remain open; the
nine research ideas stay deferred.

## Flat graph correction — 2026-10-03

This qualifies for the small-fix exception: the user explicitly rejected 3D;
the existing accepted Cytoscape renderer already supplies the replacement.
Only the renderer adapter, unused dependencies and current documentation change.
No new interface, extraction, graph authority or meaningful error state is added.
The prior pack's 3D delivery evidence remains historical. The correction is
implemented and deployed to the existing local stack. The owning visual epic
remains active; TV acceptance and overall closeout remain separate.

Dated proof: web build/typecheck/design and dependency audit pass; no production
vulnerabilities remain. Independent source and screenshot review found no
remaining blocker. The real SWEG Evidence graph has 111 entities/86 edges;
Contributions has 33/54. Both inspectors preserve explicit neighbourhood focus.
Across 29 successful pipeline refreshes, read-only browser inspection confirmed
the exact same Cytoscape instance, zoom, pan and node positions. No new tests,
model calls, live fixtures, release, commit or push were added. The removed 3D
chunk and its 43 dependency packages no longer ship. The existing 2D canvas
remains lazy (about 144 KB gzip); the pre-existing shell chunk warning remains.
Screenshots: `.cache/graph-flat-evidence.jpg` and
`.cache/graph-flat-contributions.jpg`; build, validation and deployment logs use
`.cache/graph-flat-*`. Final validator, whitespace and CodeGraph checks complete
the correction's proof; graph data and authorization rules are unchanged.

Graphify assessment: Context7 and the primary
[viewer source at commit 0b60d47](https://github.com/Graphify-Labs/graphify/blob/0b60d47e6cd9338c51143f39f35b6c45c8453385/graphify/exporters/html.py)
confirm a standalone generated HTML report using vis-network 9.1.6, static
embedded data, array-index edge IDs and a hardcoded dark theme. Its compact
search/neighbour treatment is useful; embedding the report unchanged would not
supply Recollect's authorized refresh, expiry or evidence inspection lifecycle.
Both source researcher and UI reviewer recommend the existing Cytoscape adapter
for this correction. No Graphify code, extraction runtime or dependency is added.
The lookup of a main branch failed; the repository's actual v8 branch and pinned
source above were verified instead. License/NOTICE were inspected for reuse
feasibility, not treated as authorization to install its assistant skill.

## Compact 2D graph interactions — 2026-10-03

Shared finder/type filters, Auto/All/None labels and readable hover titles,
direct-connection focus/overview restoration and compact qualified summaries
are delivered in the existing local stack. Canonical evidence, full native
paths, exact identities and failure/expiry gates remain authoritative.
The [archived pack](../execution/archive/desktop-graph-interactions.md) records
real loaded counts, 46 successful stable refreshes, path/failure checks,
desktop bounds and independent source/visual review. No new dependencies,
provider calls, tests or live fixtures. Final repository checks and CodeGraph
sync pass; version N/A, work uncommitted. TV acceptance and overall epic closeout
remain separate.

## Guided setup and automatic privacy — 2026-10-04

One-stage Plugin/Direct MCP setup, compact private context, one Add connection
chooser, qualified observed session status and explicit Pause/Enable use are
implemented and deployed locally. Privacy summarizes actual capture, storage
and per-class retention with optional separate canonical edits. Existing custom
policies and independent tool grants remain intact; saving retention keeps its
customization drawer open. The [archived pack](../execution/archive/desktop-guided-setup-and-controls.md)
records generated proposals, independent source and test-runtime screenshots,
seven existing local UI cases, backend qualification proof and final checks.
No new dependencies or test files. Ambient TV acceptance and overall epic
closeout remain open; version N/A, work uncommitted.

## Setup and exception management — 2026-10-04

The approved Agents, Connections and Settings simplification is implemented and
deployed to the existing local installation. Agents presents observed contributors
and scoped actual activity; private contexts and capture receipts are secondary.
Connections retains independent tool rights and observed successful-call timestamps.
Settings uses General, Access, Privacy and AI permissions, with preserved granular
policies and optional disclosed defaults. Closed diagnostics stop their history
feeds while access refresh remains active. The [archived pack](../execution/archive/desktop-management-simplification.md)
records exact read boundaries, focused backend/browser checks, failed-refresh and
nested-dialog proof, 37-second request observation and independent actual UI review.
No dependencies, release, commit or push. Ambient TV acceptance and overall epic
closeout remain open; the research backlog stays deferred.

## Final acceptance and closeout — 2026-10-04

- Planned: Complete ambient TV lifecycle acceptance, authorized final polish and measured initial loading; retain the original light theme and flat graph.
- Shipped: Every visual slice is accepted locally and the stack rebuilt. TV uses canonical deadlines and exact revisions, clears protected cards/timestamps on failure or expiry, preserves qualifications and supports pause/reduced-motion/exit. Graph secondary tools, compact searchable Connections, setup activity destination, direct Privacy sections and lazy presentation loading are delivered in their existing files. [Dated evidence](../../mappings/desktop-visual-closeout-2026-10-04.md).
- Not shipped: The deferred research backlog, SSE and LongMemEval are separate work; no 3D, new dependency, file-size partitioning or external publication.
- New blockers: None in the visual epic.
- Docs updated: Contract, TV/final-polish archived packs, active/archive/epic/evidence indexes and `CONTINUE_HERE.md`.
- Validation: Seven focused browser journeys across targeted runs, canonical retention test, current independent source/visual review, build/typecheck/design, clippy, governance/whitespace, CodeGraph sync and local deployment/readiness/assets. Entry 842,411 → 607,350 bytes; actual Brain chooser still fetches 755,589 bytes across 14 chunks, so no equivalent route speed or network reduction is claimed.
- Version: N/A; no release policy or version bump.
- Commit: Uncommitted; no commit, push or external release.

Earlier dated closeouts preserve their original state. The TV and final-polish
packs are now in the archive, and there are no unfinished visual slices.

## Actionable management correction closeout — 2026-10-04

- Planned: Address the user's nine annotated management issues and make optional environments actionable.
- Shipped: Neutral connection setup with actual installation catalogue provenance; explicit bounded metadata Test; compact tool inspection; member list/popups; inline Privacy Save/Cancel without a drawer or secondary tabs; managed automatic learning summary and conditional legacy overrides; optional environment creation/rename; concise private-network execution guidance. Existing custom policies, grants and live inventory remain intact. Locally deployed; [dated evidence](../../mappings/desktop-actionable-management-2026-10-04.md) and [archived pack](../execution/archive/desktop-actionable-management.md).
- Not shipped: Generic authenticated/private probes, automatic tool calls during Test, new APIs/dependencies or atomic storage-policy CAS. Research and LongMemEval remain separate.
- New blockers: None within this correction or the visual epic.
- Docs updated: Desktop contract amendment, archived pack, dated mapping, indexes and handoff.
- Validation: Six existing focused browser journeys across disposable runs; build/typecheck/design; final governance/whitespace and synchronized CodeGraph; independent source and deployed screenshots; local readiness. Earlier failed environment-label locators were corrected while retaining accessible-name assertions, and the complete journey passes. Entry remains about 607 KB / 191 KB gzip; no new route speed claim.
- Version: N/A; commit: uncommitted, no push or external release.

## General and bounded Privacy closeout — 2026-10-06

- Planned: Generate concepts first, implement selected General/environment
  hierarchy and clarify bounded Privacy rows; exercise a limited local demo.
- Shipped: General capped at 1200 px, Brain identity and environments edited
  inline, compact status/deletion rail; Privacy capped at 1120 px, thin row
  separators/focus highlight and no manual exclusion controls. Saved exclusion
  arrays, automatic redaction, existing commands and archive/permission gates
  preserved. Reader controls and actual demo agent/group access proven locally.
- Not shipped: New policy/schema semantics, backup scheduling, SDK compatibility
  repair for DeepWiki/Microsoft Learn, or unrelated native-token host acceptance.
- New blockers: None for these two slices. Full image rebuild disk guard was
  respected; frontend-only delivery used the exact existing backend image.
- Docs updated: Desktop contract, archived General pack, indexes, handoff and
  [dated evidence](../../mappings/settings-demo-scenario-2026-10-06.md).
- Validation: Type/design/build, four pure Privacy checks, regression discovery,
  32 governance tests, whitespace, CodeGraph, ready local deployment, CUA
  owner/reader/cancel/width proof and independent source/pixel acceptance.
  Canonical demo access denials and an actual anonymous Exa search succeeded.
- Version: N/A; commit: uncommitted, no push or external release.

## Six approved management concepts closeout — 2026-10-05

All six Connections, setup, global Connectors, Privacy, AI permissions and Agents
concepts are implemented, rebuilt locally and independently visually accepted at
1440 × 900 and 1920 × 1080. Twelve initial review findings were resolved, including
persistent form actions, compact inspector/sidebar, real library context, stable
Privacy editing, visible AI content/coverage exceptions and compact Agent utilities.
Inert multi-format imports and explicit central credential provisioning use bounded
authorized interfaces; execution and independent tool grants remain separate.

The [archived pack](../execution/archive/desktop-management-concepts.md) and
[dated evidence](../../mappings/desktop-management-concepts-2026-10-05.md) record
12 focused browser cases, credential/runtime proof, Clippy, build/typecheck/design,
182 API operations, 32 governance tests, CodeGraph, whitespace, exact visual-review
provenance, readiness and preserved inventory. Final subagent browser routing
failed; independent final pixel review used parent live captures, with isolated
interaction proof recorded separately. No agreed concept work remains.

The explicit installation-local file provider is a development capability;
existing Vault/OS/environment aliases remain operator managed. No external
release or tool/provider success is implied. Version N/A; work uncommitted.

## Management visual correction — 2026-10-05

The user rejected the implemented visual match after the before/vision/after
comparison and requested a UI agent to match the approved styling, formatting,
icons and edit controls. The original concept pack retains its delivery history;
its visual acceptance is superseded by this rejection. The
[archived fidelity pack](../execution/archive/desktop-management-vision-fidelity.md)
records the correction and independent reinspection. Existing behavioral
contracts and authorized inventory remain authoritative.

The UI agent corrected all six final variants. The independent reviewer accepted
fresh actual pixels at the reference size and both practical desktop sizes after
rejecting earlier rounds; all lower controls remain reachable. Twelve focused
browser cases passed together, including normal Capture clicks, stale/cancel
flows, credential/config gates, inspector actions and transient authorized Brain
navigation. The comparison retains the rejected state and embeds 88 unchanged
source images. [Current evidence](../../mappings/desktop-management-vision-fidelity-2026-10-05.md)
separates visual, functional and local-runtime proof. Work is local/uncommitted;
version N/A. No external publication or connection success is implied.

## Browser feedback closeout — 2026-10-05

All approved scale, diagnostics, agent inspection/context hiding, readable code and inline connection/tool-group/AI editor changes pass current CUA and independent actual-pixel review. Tool-group icons sit beside names; effective and Direct edit permissions remain distinct. Earlier management concepts remain historical delivery.

[Coordinated delivery, validation and limits](../../mappings/desktop-browser-management-2026-10-05.md). Version N/A; work uncommitted, no push or external release.

## Visible management follow-up closeout — 2026-10-06

- Planned: Correct all seven new marked navigation, description, AI and Privacy issues.
- Shipped: Bottom Team menu row, removal of redundant roster/read-only metadata,
  structured legacy tool descriptions and shared syntax display, aligned provider
  identity, visible AI/Privacy controls, hidden empty exclusions and optional
  coverage diagnostics. Existing automatic recovery or safe idle behavior audited.
- Not shipped: New paid retries, policy changes or unrelated native-token host acceptance.
- New blockers: None for this follow-up; separate OpenCode service limit retained.
- Docs updated: Accepted desktop/managed/semantic amendments, archived
  [pack](../execution/archive/desktop-visible-management-settings.md),
  [evidence](../../mappings/desktop-visible-management-settings-2026-10-06.md),
  indexes and handoff.
- Validation: Two pure formatting checks, frontend build/type/design,
  governance/whitespace, CodeGraph, ready local stack with unchanged 16/35/2/1
  inventory, CUA real screens and cancelled drafts. Independent final source and
  saved-pixel review accepted all seven points after two formatter findings were
  fixed. Recovery evidence is source audit, not fresh live-provider proof.
- Version: N/A; commit: uncommitted, no push or external release.

## Unified Privacy form closeout — 2026-10-06

- Planned: Replace separate Privacy cards/actions with one form and concise title separators.
- Shipped: Shared Edit/Save/Cancel, all existing retention/capture/storage fields,
  individually labeled Questions/Replies switches, no Activity/backup explanation
  text or footer AI link. Fresh-read guards, acknowledgments and explicit partial
  retry preserve existing command boundaries; retention saves last.
- Not shipped: Cross-domain atomic API, backup scheduling or unrelated native-host acceptance.
- New blockers: None for the visual epic; existing OpenCode service limit remains separate.
- Docs updated: Desktop/managed amendments, archived
  [pack](../execution/archive/desktop-unified-privacy-form.md),
  [evidence](../../mappings/desktop-unified-privacy-form-2026-10-06.md), indexes and handoff.
- Validation: Four pure save/draft checks, type/design/build, 32 governance tests,
  whitespace, CodeGraph and healthy local rebuild. CUA four-domain Cancel and
  unchanged policy SQL; independent final source and labeled pixels accepted.
  Activity-detail expiry and recovery archive windows audited in source;
  available tooling does not prove a running backup schedule.
- Version: N/A; commit: uncommitted, no push or external release.

## Connection inspection follow-up closeout — 2026-10-06

- Planned: Address marked Connections metadata/loading/formatting and Exa Test concerns.
- Shipped: Clear actual badges/read states, formatted descriptions and JSON, structured
  call details and bounded equivalent schema inspection on the ready local stack.
- Not shipped: Broad schema migration, automatic tool execution or new grants.
- New blockers: None for this follow-up; existing native-host work stays separate.
- Docs updated: Contracts, archived packs, indexes, handoff and [current evidence](../../mappings/mcp-inspection-readability-2026-10-06.md).
- Validation: Pure/owned backend fixtures, build/lint/governance, actual CUA read/check/
  expiry/width proof, preserved runtime invariants and independent source/pixel review.
- Version: N/A.
- Commit: Uncommitted.

## Compact Tool access closeout — 2026-10-06

- Planned: Implement the approved tighter cards and independently verify appearance/function.
- Shipped: Compact header/filter, aligned bounded cards, one adjacent effective
  icon set with locked inheritance and concise source/direct popover; private
  reader row and existing canonical edit/retry retained on the ready local stack.
- Not shipped: New backend policy, external OIDC live fixture or unrelated native-host work.
- New blockers: None for this slice or visual epic.
- Docs updated: Contracts, archived [pack](../execution/archive/desktop-compact-tool-access.md),
  indexes, handoff and [dated evidence](../../mappings/desktop-compact-tool-access-2026-10-06.md).
- Validation: Eight focused permission tests, type/design/build, governance32,
  CodeGraph, actual CUA draft/Cancel/reversible Save/partial retry, unchanged
  canonical inventory and demo Use-only/two-tool proof passed. Independent final
  source and actual 1384/2504 px comparison accepted without remaining blockers.
- Version: N/A.
- Commit: Uncommitted.

## Tool access header badge correction — 2026-10-06

- Planned: Separate environment/MCP metadata into badges and fix the edit picker position.
- Shipped: Two compact badges beneath the title; the environment picker replaces
  its badge in the same row, aligned with the MCP count. Ready local frontend.
- Not shipped: Backend/configuration or permission changes; none requested.
- New blockers: None.
- Docs updated: Contract, small-fix slice/index, dated evidence and handoff.
- Validation: Type/design/build, governance32, whitespace, CodeGraph and actual
  CUA read/edit/picker/Cancel checks passed; measured title/picker/count alignment.
  [Follow-up evidence](../../mappings/desktop-compact-tool-access-2026-10-06.md#header-badge-follow-up).
- Version: N/A.
- Commit: Uncommitted.

## Private runner naming and concepts — 2026-10-06

- Planned: Rename the tab and generate matching read/inline Add/Edit concepts.
- Shipped: Private Runners tab/guidance on the ready local stack; saved read and
  tightened inline concept images with fictional sample data and exact prompts.
- Not shipped: Inline editor implementation, new registrations or live runner proof.
- New blockers: None for the requested label/concept deliverable.
- Docs updated: Contract, small-fix slice/index, mappings/index and handoff;
  [evidence and limits](../../mappings/private-runners-concepts-2026-10-06.md).
- Validation: Web type/design/build, governance32, whitespace, CodeGraph and
  actual CUA selected-tab/unchanged empty registration view passed.
- Version: N/A.
- Commit: Uncommitted.

## Inline Private Runners delivery — 2026-10-06

- Planned: Implement the approved read/inline concepts and explain runner versus agent roles.
- Shipped: Bounded light cards, compact name/device/status, inline Add/Edit,
  locked existing host, explicit setup copy and canonical guarded save/retry;
  ready local frontend. [Current evidence](../../mappings/desktop-inline-private-runners-2026-10-06.md).
- Not shipped: New real registrations, worker startup or new backend behavior.
- New blockers: None for this slice.
- Docs updated: Contract, archived pack, epic/index, mappings/index and handoff.
- Validation: Web type/design/build, required governance32, whitespace and
  CodeGraph passed; CUA live empty/Add/Cancel and fixture Save/Cancel/retry/stale/
  device/reader/archive checks plus actual laptop/wide comparison passed.
- Version: N/A.
- Commit: Uncommitted.

## Private Runners spacing follow-up — 2026-10-06

- Planned: Fix the route's spacing first and explain a clearer runner relationship display.
- Shipped: Left-aligned bounded page and consistent tab-to-toolbar gap on the
  ready local frontend. [Evidence and proposal](../../mappings/private-runners-spacing-2026-10-06.md).
- Not shipped: The proposed assigned-connections/tools/tool-groups section;
  no new runner behavior or permission projection.
- New blockers: None for spacing. A reader-safe relationship projection needs
  a separate decision and implementation because connection detail is admin-only.
- Docs updated: Contract, this small-fix slice/index, mapping/index and handoff.
- Validation: Web build/type/design, CUA read/Edit/Add/Cancel at laptop/wide
  sizes, independent visual review, readiness, governance32 and whitespace.
- Version: N/A; presentation correction without a release policy.
- Commit: Uncommitted; no commit, push or external release.

## Private Runner relationship cards closeout — 2026-10-06

- Planned: Implement the approved device/MCP/tools/group hierarchy with independent UI review.
- Shipped: Compact relationship cards with canonical reader-safe binding summaries, shared
  definition/profile metadata and exact resource links, deployed to the ready local stack.
  [Evidence](../../mappings/private-runner-relationships-2026-10-06.md),
  [archived pack](../execution/archive/desktop-private-runner-relationships.md).
- Not shipped: New grants, execution, telemetry or device rebinding are outside this slice.
- New blockers: None. Earlier spacing proposal is superseded by this approved delivery.
- Docs updated: Contracts, epic/index, pack/index, evidence/index and handoff.
- Validation: Two Rust tests, web type/design/build, 13 CUA state/pagination/pending/link
  checks, actual owner/reader APIs and rendered UI, Add/Edit/Cancel, independent
  laptop/wide review, governance32, CodeGraph, readiness and whitespace passed.
- Version: N/A; commit: uncommitted. No push or external release.


## Processing limits and input error follow-up — 2026-10-07

The explicit placement correction qualifies as a small isolated presentation
fix: canonical fields, bounds, authority and shared policy commands are unchanged.
Processing limits now sits below Automatic processing with a two-column grid,
an accessible heading info icon and no permanent helper paragraph. Header
actions wrap rather than shrink. Browser open-help and Cancel are verified.

The subsequent cross-module error work has its own
[archived pack](../execution/archive/desktop-input-errors-and-page-recovery.md)
and [dated evidence](../../mappings/input-errors-and-page-recovery-2026-10-07.md).
Actual learning rejection is distinct from capture, budget-blocked indexing no
longer recreates minute-by-minute attempts, and a missing chunk gets safe page
recovery. Local deployment is healthy; no release or commit was requested.

## Dashboard refresh closeout — 2026-10-07

Fresh activity content stays mounted across brief visibility changes and Brain
metadata revisions. Hidden motion pauses; role changes and failed/expired reads
still clear protected state. [Pack](../execution/archive/desktop-dashboard-refresh-stability.md)
and [evidence](../../mappings/dashboard-refresh-stability-2026-10-07.md) record
reproduction, exact graph DOM/inspection checks, six live polls, build and
local readiness. No new cache deadline, server change, release or commit.
