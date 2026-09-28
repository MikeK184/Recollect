# Recollect desktop experience plan

Status: accepted
Prepared: 2026-09-26
Accepted: 2026-09-26, by the user's explicit approval and request to implement all of it.
Scope: approved desktop implementation plan and illustrative design concepts.
Version: N/A; the uncommitted implementation is deployed locally, with remaining domain acceptance in progress.

[ADR 0014](../../adr/0014-desktop-experience-and-answers.md), the
[desktop contract](../../contracts/desktop-experience.md) and
[answer contract](../../contracts/retrieval-answers.md) record the accepted
implementation details. [Active packs](../execution/active/README.md) track work;
design approval is not a shipment or runtime claim. The user also explicitly
requested one original reusable Recollect SVG logo, inspired by gathering memories
again, with consistent SVG/icons, fonts, sizes and headings throughout the product.

## Recommendation

Make Recollect a place to ask questions, inspect what it knows, and configure
standing permissions. Routine capture, retrieval, scope management, learning,
and maintenance should remain agent/background work.

Keep React, TypeScript, Vite, Mantine, TanStack Router/Query, Cytoscape, and the
Rust same-origin API. Introduce a reusable light-only design system based on the
Atlas palette and typography. Replace the single long Brain page with nine
contextual routes. Add evidence-backed Ask as a separately specified backend
capability, with existing evidence search always available.

The current UI makes real, useful capabilities feel like controls that a person
must operate continuously. Most are not UI-only implementations: they already
call governed backend handlers. What is unnecessary is their everyday prominence.
Changing where controls live must preserve access, provenance, correction,
expiry, and agent authority.

## Review package

- [Implemented desktop guide](../../runbooks/desktop-experience.md) and
  [dated implementation evidence](../../mappings/desktop-experience-implementation-2026-09-26.md):
  current source/assets, verified final local image and completed checks, with
  remaining domain acceptance explicit.

- [Visual gallery](index.html): all 12 desktop concepts, with page purpose and
  implementation notes. These are generated design images, not functioning
  screens or live product data.
- [Current UI audit and complete function map](current-ui-audit.md).
- [Agent capability and Ask design](agent-and-ask-design.md).
- [Design system and framework recommendation](design-system.md).
- [Dated evidence and limits](../../mappings/desktop-experience-review-2026-09-26.md).
- [Image prompts](prompts.json): shared shell plus one prompt per view, generated
  with the built-in image generation tool.
- [Image inventory](#screen-inventory): every proposed main navigation destination.

Three independent read-only reviews covered UI/function ownership, Rust/API/MCP
capabilities, and Atlas/competitor/design-system evidence. This plan synthesizes
their findings. The user supplied the visual direction and requested the review,
plan, multi-agent work and images, then explicitly accepted the result for
implementation. Domain contracts govern actual behavior.

## Findings that drive the change

1. Four current routes contain almost the entire product in one Brain page:
   sources, access, retrieval, graph, tasks, connections, publication, capture,
   memory, models, handovers, retention, jobs, and audit. There is no useful
   feature-level browser history or deep linking.
2. Autonomous maintenance is already accepted and implemented. Normal use does
   not require a human to approve each learned memory, create a task, rebuild an
   index, or press a learning button.
3. Current Recall returns bounded evidence, not a generated answer. Ask therefore
   needs answer orchestration, separate model permission, eligible evidence
   inputs, citations, failure handling, and conversation-lifetime rules.
4. Modes express real eligibility choices. Hide mechanics until needed, but
   continue showing scope and any non-default/historical criteria.
5. Graph should open with a useful bounded visualization. The current filters,
   graph rebuild, paths, and hop parameters push the visualization below a wall
   of controls.
6. Agent connection, device identity, and outbound tool connection are different
   concepts. Their setup is currently scattered; merging them into one vague
   Connect page would make authority harder to understand.
7. Optional human correction, withdrawal, erasure, and evidence inspection remain
   necessary. Automation does not make erroneous memory impossible.
8. No framework replacement is justified by these issues. Atlas's typography and
   palette can be expressed centrally in the current stack.

## Navigation and routes

```text
Workspace
├── Brains                       /
├── Team                         /team
└── Devices                      /devices

Inside a Brain                  /brains/:brainId
│   Brain switcher + All Brains
├── KNOWLEDGE
│   ├── Ask                      /ask
│   ├── Memory                   /memory
│   ├── Sources                  /sources
│   └── Graph                    /graph
├── WORKSPACE
│   ├── Repositories             /repositories
│   └── Agents                   /agents
└── MANAGE
    ├── Connections              /connections
    ├── Activity                 /activity
    └── Settings                 /settings
```

The sidebar changes with location. Global navigation does not remain as a second
full sidebar beside the Brain menu. The Brain switcher and All Brains link provide
the return path. The topbar shows the current Brain and page; permissions govern
the actions and data inside each page. Team remains installation-owner only.

Final target: opening a Brain selects Ask. During migration, the parent route
opens the functioning evidence-search view until answer synthesis ships. Do not
expose a nonfunctional answer composer while the backend is incomplete.

Each page owns a URL. Meaningful tabs use validated URL state, e.g.
`/repositories?tab=environments`, `/memory?kind=handover`, and
`/connections?tab=profiles`. Resource details gain stable same-Brain child URLs
or validated identifiers, preserving exact revisions and return location.

Shareable non-sensitive scope/filter choices may appear in the URL. Question
text, generated answers, retrieved content, private local paths, credentials,
and tokens do not. Changing Brain clears conversation/evidence state and any
incompatible scope; it never silently maps IDs between Brains. Back/forward,
reload, direct links, forbidden resources, archive state, and lost access are
first-class behaviors. A page has a clear loading/error state instead of old
data from the previously selected Brain.

## Page-by-page specification

### Ask

Purpose: ask a natural question and receive a qualified answer with its sources.

- Default: question composer, visible Brain and meaningful scope chips, answer
  area, citations, and a supporting-evidence inspector.
- Primary action: Ask. Secondary action: Search evidence.
- Existing search remains a distinct view with Matches / Disagreements / Sources
  and Copy context. Exact identity lookup and advanced criteria remain available.
- Keep result limits, budgets, retrieval modes, fact/knowledge time, exact
  manifests, and channel detail in Advanced search. Show applied constraints
  above every result; a collapsed drawer must not conceal a narrowed selection.
- No model dropdown or retrieval algorithm choice before an ordinary question.
  The Brain's approved answering policy determines model/provider and budget.
- First version: temporary conversation held only in current browser memory.
  A follow-up retrieves fresh evidence. Prior assistant prose is not evidence.
- Asking does not publish memory, run an MCP tool, start a deployment, or create
  a persistent task. Retrieved instructions are data, not tool authority.
- Answering disabled/unconfigured: present exact/text evidence search and explain
  what an admin can configure. Insufficient support: explain the missing support
  and retain useful partial evidence. Conflicts stay visible.
- Loading shows bounded stages; initial release displays a validated completed
  answer, not unvalidated token streaming.
- Citations identify the evidence actually supplied, not a promise of universal
  truth or deployment proof. See the dedicated Ask design for enforcement.

### Memory

Purpose: read learned knowledge and intervene when useful.

- Tabs/types: All, Decisions, Procedures, Handovers; ordinary claims remain under
  All with type filters. Search and a nonmandatory attention filter. The new
  list search needs authorized server-side filtering before pagination; it must
  not search only the loaded page or imply an exhaustive Recall response.
- Main list: title/assertion, type, concise status, applicability, source count
  when supported, and last known update. Avoid a spreadsheet of internal IDs.
- Detail inspector: readable content, exact support, separate review/freshness/
  operational states, fact versus knowledge time, history, correction, withdrawal,
  rejection/conflict resolution, and contextual erasure.
- Add memory is secondary; it does not imply all knowledge must be entered by hand.
- Handovers tab includes the existing generated-handover workflow, selecting
  exact revisions of claims, decisions and procedures, with their supporting
  sources inspectable, generation status, retry/refresh, and contributor scope.
  No silent auto-save of Ask output as a handover.
- Missing source, disputed memory, archived Brain, reader rights, stale revision,
  and erased/expired content receive explicit states.
- Do not create a mandatory daily review inbox.

### Sources

Purpose: manage original material that supports memory.

- Primary action: Add source, with current text-file/paste/reference capabilities
  accurately described. Do not advertise arbitrary PDF/OCR/connectors without
  supporting import work.
- Search/list: title, kind, availability, last update, processing/learning state.
  Text search is new list-API work: filter authorized records before pagination,
  with clearly specified title/content scope and no model call. Counts and
  learning badges require verified response support.
- Collections, areas, environments: compact filters and one Manage views editor.
  Removing a grouping/association is not erasure of shared evidence.
- Source detail: readable content or reference-only explanation, exact versions,
  attribution, associations, edit as a new version, linked derived memory.
- Rare actions: reprocess, explicit learning, technical receipts, erase preview.
- Import shows the current storage mode and links to the canonical retention
  setting; a global storage toggle does not occupy the everyday sources list.
- Useful states: empty, importing, processing, failed, retained, reference-only,
  unavailable bytes, and permission/retention limitation.

### Graph

Purpose: explore relationships and inspect why they exist.

- Canvas is the main desktop area, not the final panel below a form.
- Compact toolbar: Knowledge / Repository / Combined, entity search, current
  environment/revision scope chips, Filters, and accessible list alternative.
- Existing exact selection rules remain. Never select a guessed environment
  manifest or silently turn a historical investigation into a current graph.
- A safe bounded knowledge overview may load on entry under a new route contract;
  no model call, rebuild, heavy analytics, or arbitrary traversal on navigation.
  If exact repository inputs are missing, ask for the required selection.
- Right inspector: node/edge identity, supporting evidence, applicability,
  relationships, and Open source. Find path chooses endpoints contextually.
- Advanced drawer: eligibility, relations, dates, direction/hops, exact manifests.
  Insights runs the existing bounded native analytics explicitly.
- Small status popover: coverage, readiness, generation/epoch, unresolved links,
  jobs, diagnostics, and authorized rebuild. These are not main-page forms.
- Keyboard entity navigation and searchable list/table are required alongside
  canvas controls. Partial, stale, unavailable, and missing-center states must
  not fabricate edges or widen scope.

### Repositories

Purpose: inspect published code knowledge and environment revision selections.

- Tabs: Repositories and Environments. Show canonical repository origin,
  snapshot/revision, publication time, and extraction coverage.
- Detail: snapshot Files / Facts / Coverage / Contributors / Insights / Receipt,
  associated memory, origin aliases, and authorized reprocess/erase.
- Publish from companion opens accurate native instructions; the browser does
  not suddenly gain local filesystem scanning or Git clone authority.
- Environments: exact revision manifests, create/edit selections, history,
  committed versus desired versus observed status; this is not deployment.
- Your checkouts remains an account/device-specific inspector. Local paths do
  not become Brain-wide because a repository is shared.
- No silent latest-commit substitution; incomplete extraction stays visible.

### Agents

Purpose: connect coding tools to Recollect and inspect supported activity.

- Connect an agent guides through supported host, companion pairing if needed,
  Brain binding, workspace configuration, host setup, and a real memory-read check.
- Supported host setup cards are not evidence that an agent is currently running.
  Live/last-seen indicators require real server observations.
- Tabs: Sessions, Capture, Task scopes. Session views link to retained sources
  and show coverage/gaps; task scope details are personal to their account.
- Tasks mean immutable scope/binding contexts for agents, not a human to-do app.
  Creation, child scopes, operation binding/history, and closing remain available
  as advanced controls. Normal agents already manage these through MCP.
- Capture configuration has one canonical editor under Settings; this page
  displays effective policy/coverage and links there.
- Replace prominent Refresh catalogue with last-updated/error retry. Native
  workspace discovery is separately labelled and does not imply extraction.
- Distinguish personal task metadata from captures deliberately published as
  Brain knowledge. A Brain admin does not acquire everyone else's private tasks.

### Connections

Purpose: configure external tools that authorized agents can call.

- Tabs: Connections, Profiles, Runners.
- Add connection wizard: approved connector → target/environment → credential
  reference → execution location → review → explicit test.
- No approved definition: explain the operator setup requirement. Do not present
  arbitrary executable installation as a current browser feature.
- Profiles select connections and independently grant Use, Manage, Share.
  Brain admin/owner is not automatically a tool user.
- Runners select central/native/private placement and link to paired device
  identity. Secrets remain referenced, never displayed in a list or mock fixture.
- Detail shows configured state separately from observed last successful call,
  tool schemas and calls, credential health, edit/revoke, and explicit test.
- Manual tool calls stay in a troubleshooting inspector with their real effect
  and permissions; labeling a call Test must not imply it is read-only.
- Uncertain outcome, reconciliation, cancel, resolution, and session release keep
  their existing semantics; no automatic retry of a potentially completed write.

### Activity

Purpose: understand outcomes and fix exceptions, without becoming a work queue.

- Tabs: Timeline, Processing, Tool calls, Model usage; optional subfilters for
  learning, capture, changes, and data removal.
- Timeline summarizes timestamped events and links to canonical detail screens.
  It must not claim a total causal order across independent subsystems.
- Processing shows queued/running/partial/failed states and appropriate
  retry/cancel controls; irreversible/uncertain work retains its restrictions.
- Model usage shows actual requests/tokens/budget metadata and failure causes.
  Tool calls preserve profile authority; administrative audit remains admin-only.
- Data-removal detail preserves erase progress and backup/restore limits.
- Render only authorized feed types. Account-private scopes/paths are never
  leaked through aggregation. Reader access to normal knowledge does not imply
  access to admin audit or arbitrary caller outputs.
- Initial implementation may compose the existing bounded APIs. If a unified
  paginated feed, joined status/counts, or query efficiency needs a new read
  endpoint, scope it explicitly; do not fake those fields client-side.

### Settings

Purpose: make standing human decisions understandable and editable in one place.

- General: Brain name/description; archive/reopen separated from ordinary editing.
- Access: effective members/roles, ownership and independent grants explained.
- AI & automation: automatic learning and permissions for the deployment's
  installed models, purposes, selected content classes, model transmission,
  answering permission, semantic search, token/concurrency limits, and model
  connection check. Current provider/model fields are deployment-selected and
  read-only; adding/selecting further providers is separate operator/API work.
- Capture: supported event kinds and exclusions, retention, enabled/paused state;
  separate recording permission from external model transmission.
- Retention & privacy: documents, source excerpts, snapshots, captures, memory
  forms, handovers; readable policy summary, impact explanation, detailed editor.
- Rare maintenance: semantic rebuild, policy history, technical status.
- Existing installations retain their policies. New answering permission starts
  off; no toggle may silently enable transmission or change data retention.
- Readers see applicable policy summaries where appropriate; only actual
  authorized roles can change them. Unsaved changes and stale-policy versions
  need clear recovery.

### Global Brains, Team, Devices

- Brains: create/open, personal/shared and archived views, search, concise useful
  metadata. Avoid unimplemented scorecards. Installation health is a small
  affordance; owner diagnostics open in a drawer.
- Team: installation accounts, local invitation or OIDC enrollment, invitations,
  enable/disable, reset sign-in, and account audit. Brain sharing is separate.
- Devices: your paired identities, native pairing instructions, pending approval,
  deny/revoke and last-seen if available. Pairing approval remains explicit.
- Sign-in, invite acceptance, expiry, OIDC errors/recovery, and sign-out inherit
  the new tokens and interaction rules, though they are not sidebar destinations.

## Defaults, automation, and human intervention

| Activity | Default owner | Human UI |
| --- | --- | --- |
| Select task/subagent working scope | Agent using existing scoped tools | Inspect/override own context when needed |
| Capture supported permitted events | Companion under standing capture policy | Setup, coverage, pause/configure |
| Learn, reconcile, revise, retire memory | Existing autonomous workers under policy | Configure once; optional inspection/correction |
| Retrieve relevant context | Agent via MCP; Ask service for browser questions | Ask or inspect exact evidence |
| Graph/semantic maintenance | Existing bounded jobs | Status and repair on demand |
| Publish committed repository snapshots | Authorized companion/native workflow | Setup and inspect; no pretend browser scan |
| Allow installed models/purposes and budgets | Authorized human/admin | Guided settings; provider installation remains deployment configuration |
| Pair devices and grant Brain/profile access | Authorized human/admin | Explicit setup and revocation |
| Configure tool credentials/targets/runners | Authorized human/operator | Connection wizard; approved catalogue |
| Correct/withdraw/erase memory | Canonical policy plus authorized explicit intervention | Reachable contextual controls with real consequences |

There is no requirement for complete UI/MCP administrative parity. Existing
agents need memory and scoped tool access, not authority to self-grant permissions,
approve executables, or change data-sharing policy. The capability audit names
useful future gaps without granting this broader authority.

## Architecture and reusable frontend structure

Use primitive → semantic → component tokens, a light-only Mantine theme, shared
components, and route-owned feature views. Self-host Newsreader, Manrope, and
DM Mono with license notices. Keep the existing generated OpenAPI client and
canonical backend handlers.

```text
web/src/
├── design/              tokens, theme, typography, component defaults
├── app/                 router, session boundary, WorkspaceShell, BrainShell
├── components/          PageHeader, FilterBar, ScopeSummary, DetailInspector,
│                        StatusBadge, AsyncState, SettingsSection, ActionMenu
└── features/
    ├── ask/
    ├── memory/
    ├── sources/
    ├── graph/
    ├── repositories/
    ├── agents/
    ├── connections/
    ├── activity/
    └── settings/
```

Split data hooks from page composition; do not duplicate mutations across routes.
Use one canonical evidence/claim/connection inspector wherever linked. Existing
dialogs may be wrapped during migration. Lazy-load route bundles and mount only
active-page polling, plus the shell and mandatory current-content
expiry/revocation checks. Measure request volume before claiming a reduction.

Do not switch to Next.js, add a production Node service, replace Rust endpoints,
or introduce a second authorization layer just for this redesign. Cytoscape
already supports the graph interaction model; restyle and reorganize before
considering a graph-library replacement.

## Behavior changes that require explicit specification

| Proposal | Why it is more than restyling | Required owner/decision |
| --- | --- | --- |
| Ask and temporary follow-ups | New generated output, policy purpose, retrieval input and answer validation | Hybrid retrieval + model policy contracts |
| Default auto-load of bounded graph | Entry triggers a new read; selection, bounds and errors must be fixed | Graph exploration contract |
| Unified Activity/session summaries | New aggregation may join independently authorized/private data | Owning read contracts and operations |
| Guided onboarding with proof checks | A check may call a provider or tool and incur cost/effects | Agent/MCP/model owners; label explicit action |
| Source/learning aggregate badges | May need endpoints or bounded joins not currently present | Evidence/memory owner |
| Memory and Sources list search | Current paginated lists lack text-query input; add authorized filtering before pagination, not a loaded-page-only filter | Evidence/memory list contracts; title/content fields and limits specified in their slices |
| Adding/selecting model providers in the UI | Current provider/model pair is deployment-selected and read-only | Deferred operator/API capability; current UI edits permission for installed models |
| Persisted conversations | New storage, visibility, retention/erase/backup obligations | Deferred; separate future decision |
| Ask executes tools or writes memory | New command and grant semantics | Deferred; separate explicit intent and policy design |
| Browser approval of arbitrary MCP definitions | New privileged executable/catalogue authority | Deferred; current operator workflow retained |
| Additional MCP source/publication/admin tools | New agent authority and public schemas | Separate parity proposal; not implied by new UI |

A hidden advanced control retains the old default and enforcement unless the
relevant contract explicitly changes it.

## Delivery sequence and accepted slices

The nine slices below are registered in their existing owning epics and the
active execution index, with accepted authority and decision-complete packs.
They are in progress, not shipment claims. Capability UI remains with its owner;
no isolated frontend epic takes over memory/MCP authority. Dependencies sequence
integration/acceptance; independent backend or presentation work may proceed once
its own decisions are complete.

| Order / slice ID | Owner | Depends on | Deliverable and exit check |
| --- | --- | --- | --- |
| 0 / `desktop-experience-contracts` | Product platform coordinates domain owners | Plan review | Record route/state contract, design choices, Ask policy/retention boundary, API gaps; no unresolved behavior in implementation packs |
| 1 / `platform-desktop-shell` | [Product platform](../epics/product-platform.md) | 0 | Tokens, fonts, shared shell/components, global pages and real nested routes; deep links, roles and light desktop shell proven |
| 2 / `evidence-desktop-workflows` | [Evidence/workspaces](../epics/evidence-and-workspaces.md) | 1 | Sources, repositories/manifests, agent onboarding/session/context views; existing import/publish/scope/capture behavior preserved |
| 3 / `memory-desktop-workflows` | [Memory lifecycle](../epics/memory-lifecycle.md) | 1 | Memory, handovers, settings/policy/retention, canonical inspectors; autonomous operation and corrections preserved |
| 4 / `graph-desktop-workspace` | [Graph intelligence](../epics/graph-intelligence.md) | 1 | Canvas-first graph, inspector, paths, analytics drawer and bounded defaults; keyboard and stale-data proof |
| 5 / `mcp-desktop-setup` | [MCP coordination](../epics/mcp-coordination.md) | 1 | Connections/profiles/runners wizard and call inspector; grants, observed status and uncertain-effect handling proven |
| 6 / `retrieval-ask-experience` | [Hybrid retrieval](../epics/hybrid-retrieval.md) | 0,1, shared inspectors from 2/3 | Existing search migration plus new Ask endpoint/bundle/answering purpose, source citations, temporary session and fallbacks |
| 7 / `operations-desktop-activity` | [Operations](../epics/operational-readiness.md) | 2,3,4,5 | Authorized aggregate presentation and diagnostics; source links, feed boundaries, expiry/private-data tests |
| 8 / `desktop-experience-acceptance` | Operations with all owners | 2–7 | End-to-end human/agent journeys, desktop/accessibility/visual checks, performance measurement, rollback and docs closeout |

Stages 2–5 can progress independently after the shell and shared interfaces are
stable; Ask backend work can also proceed independently of purely visual screens.
Deliver the functioning navigation/theme improvements before waiting for all new
answer behavior. Keep the final default-Ask cutover behind actual acceptance.

No calendar estimate is asserted before contract/API-gap review. Relative effort:
shell/navigation medium; source/memory/MCP refactors medium to large; Ask large
because it adds product semantics; Activity size depends on read-API support.

## Migration, validation, and rollback

1. Record existing journeys and representative fixture states; preserve current
   browser behavior tests. No production data migration for a visual refactor.
2. Build tokens/components and new layout behind an installation development flag.
   Existing parent Brain links continue to resolve; a new tab/route does not
   change the selected task or model policy.
3. Move features one at a time, reusing canonical hooks/mutations and invalidation.
   Do not mount both old and new dashboards simultaneously.
4. Add any Ask policy/schema changes separately with backward-compatible defaults
   and server capability detection. Deploying new assets must not enable answers
   or model transmission on existing Brains.
5. Test 1280×800, 1440×900/960, and 1920×1080/1200 desktop layouts. No mobile or
   small-tablet redesign is part of this request.
6. Check keyboard navigation, focus return, accessible names, table/list graph
   alternative, measured contrast, reduced motion, and clear non-color states.
7. Run meaningful route/auth/domain browser regression tests plus selected visual
   comparisons. Loading, empty, partial, stale, failed, forbidden, archived,
   expired and long-content states must be usable.
8. Prove cross-Brain isolation, current permission loss, rejected/erased content
   suppression, in-flight navigation cancellation, private task scope, and
   independent profile grants. Keep positive useful results alongside negatives.
9. For Ask, run explicit citation/unsupported/conflict/provider-policy/injection/
   expiry/no-side-effect evaluation through the real path. A plausible sentence
   or valid source ID alone is not adequate proof.
10. Measure mounted requests, p95 navigation/recall/answer stages and graph
    responsiveness against current comparable fixtures. Do not invent a faster
    performance claim from component count.
11. Run frontend typecheck/build, focused API tests where semantics change,
    `./scripts/validate.sh`, and `git diff --check`; verify actual desktop
    behavior before changing lifecycle status.
12. Roll back a visual regression by serving the previous compatible frontend
    or disabling the development route flag. Disable answering separately if
    the new service fails; preserve exact/text search. Do not roll back policy/
    erasure data or restore removed permissions as part of UI rollback.

## Acceptance journeys

- Human opens Brain → asks question → sees qualified answer → opens exact cited
  source → corrects memory if authorized → subsequent human/agent recall honors it.
- With answers disabled, a reader still finds exact/text evidence without a
  model call or admin configuration ceremony.
- Engineer connects companion/host once → agent discovers workspace, starts its
  own scope, recalls/contributes, and calls explicitly granted tools without
  creating tasks or clicking rebuild in the browser.
- Administrator adds an approved MCP connection → chooses credential reference/
  placement → grants profile use separately → observes an actual call result.
- Graph user sees a bounded graph immediately when valid → selects a node →
  follows evidence or requests an explicit path/analysis.
- User moves between Brains while a request is pending → late data never
  appears in the other Brain; expired or revoked evidence is cleared.
- Operator diagnoses a failed job or uncertain tool result through Activity →
  reaches one canonical owner inspector with appropriate recovery.

## Accepted decisions

| Decision | Recommendation | Reason |
| --- | --- | --- |
| Brain landing page | Ask, with Search evidence fallback | Matches the user's question-oriented use |
| Navigation | Nine contextual Brain pages, three global pages | Covers current capabilities without one menu item per engine operation |
| Framework | Keep current stack | Already supports tokens, layouts, typed data and static deployment |
| Visual direction | Atlas-inspired light-only, app-scale typography | Matches the supplied preference without importing marketing layout |
| Ask authority | Read-only knowledge answers first | Useful first scope without implicit execution or durable writes |
| Conversation storage | Temporary initially | Avoids quietly adding permanent sensitive transcript storage |
| Human workflow | Setup once, inspect/correct optionally | Follows accepted autonomous-memory intent |
| Future extensions | Saved conversations and action-capable Ask separately | Need their own authority, retention and execution design |

The user explicitly accepted these decisions and requested complete implementation
on 2026-09-26. The accepted contracts preserve their boundaries; no further
per-slice approval ceremony is required. Runtime changes are proven by the owning
packs before they are marked shipped.

## Screen inventory

All images are desktop concepts with fictional sample data. Small generated-copy,
date, icon and spacing variations are not a specification; the written page
contracts and design tokens govern implementation. Each main sidebar destination
has its own image. Tabs, dialogs, errors, and advanced drawers are specified above
and need implementation-state designs; they are not claimed as fully mocked here.

| Image | Navigation destination |
| --- | --- |
| [01 Ask](images/01-ask.png) | Brain / Ask |
| [02 Memory](images/02-memory.png) | Brain / Memory |
| [03 Sources](images/03-sources.png) | Brain / Sources |
| [04 Graph](images/04-graph.png) | Brain / Graph |
| [05 Repositories](images/05-repositories.png) | Brain / Repositories |
| [06 Agents](images/06-agents.png) | Brain / Agents |
| [07 Connections](images/07-connections.png) | Brain / Connections |
| [08 Activity](images/08-activity.png) | Brain / Activity |
| [09 Settings](images/09-settings.png) | Brain / Settings |
| [10 Brains](images/10-brains.png) | Workspace / Brains |
| [11 Team](images/11-team.png) | Workspace / Team |
| [12 Devices](images/12-devices.png) | Workspace / Devices |

## Proposal validation

The final validation record is kept in the dated evidence mapping. Documentation
and gallery checks do not prove the redesigned product works. This deliverable
contains planning documents, a visual gallery, prompts and generated images.
Application code, databases, running-stack configuration and reference checkouts
were unchanged by the original planning task. Subsequent implementation is tracked
separately in the active packs and must supply its own runtime evidence.
