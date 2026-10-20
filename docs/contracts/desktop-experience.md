# Contextual desktop experience

Status: accepted

## Source

[ADR 0014](../adr/0014-desktop-experience-and-answers.md) as amended, the accepted
[plan](../roadmap/desktop-experience/README.md),
[design system](../foundation/design-system.md) and existing
[foundations](../foundation/README.md) govern this work. Decisions folded into
this specification, each an explicit user approval:

- 2026-09-26: the complete desktop proposal, with consistent fonts, icons and
  SVGs and an original SVG logo.
- 2026-10-01: the display and interaction priority.
- 2026-10-03: the live control panel built on actual activity, the Brains
  chooser, Brain artwork and the grouped-list Agents design.
- 2026-10-04: annotated actionable-management feedback, the
  [setup and exception management](desktop-knowledge-surface.md) simplification,
  and final polish with ambient acceptance.
- 2026-10-05: the six management concepts (delivered by the
  [concept pack](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/desktop-management-concepts.md)),
  the vision-fidelity correction and all ten items of the
  [browser review](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/research/browser-management-review-2026-10-05.md),
  with Connections-style tool-group cards replacing the rejected list/inspector concept.
- 2026-10-06: settings refinement, Privacy, connection readability, compact
  tool-access and Private Runner follow-ups.
- 2026-10-07: plugin workflow repair, main-form processing limits, page recovery
  and learning reasons.
- 2026-10-19: the two-theme redesign recorded in
  [ADR 0025](../adr/0025-two-theme-desktop-redesign.md).

The [tiered surface contract](desktop-knowledge-surface.md), decided under
[ADR 0017](../adr/0017-desktop-knowledge-and-ask-experience.md) on 2026-09-29,
governs sidebar tiering, the merged knowledge surface, the Dashboard landing
default, the Agents/Connections split and the assurance band. It changes
placement and presentation only: this contract's token, typography, icon,
route-ownership, desktop-width, accessibility and read-only-answer requirements
remain in force, and every route named below stays a real deep-linkable URL. The
[managed experience contract](memory-managed-experience.md) governs the
2026-09-28 managed setup, progressive disclosure and owner connector approval
changes; earlier explicit APIs and stored policies remain compatible. The
[credential provisioning rules](mcp-runtime-and-credentials.md#browser-managed-credential-provisioning)
govern secret entry independently of visual layout.

Approved concept images: `output/imagegen/2026-10-05-management-concepts-01a10b36`
(six management concepts), `output/imagegen/2026-10-06-settings-refinement`
(General and inline environment layouts),
`output/imagegen/2026-10-06-tool-access-compact/tool-access-v2.png`,
`output/imagegen/2026-10-06-private-runners/` (`view.png`, `inline-v2.png`) and
`output/imagegen/2026-10-06-private-runners-relationships/view-v2.png`.

The text before this compaction, with each dated amendment, is at
[revision a2338a8](https://github.com/MikeK184/Recollect/blob/a2338a80c824f00214ee33e37596e5a92a2cc409/docs/contracts/desktop-experience.md).

## Contract

### Display and interaction priority

The user prioritizes normal laptop screens and larger desktop monitors, up to
about 32 inches. Aim for comfortable desktop reading and ordinary pointer use;
physical screen size is not a fixed CSS resolution. 1440px is the smallest
supported layout (16-inch laptops and larger). Review pages at 1440×900,
1536×864, 1728×1117 and 2560×1440 in both themes, with other desktop sizes when a
changed layout needs them. Below 1440px the inspector opens as a sheet and the
sidebar collapses to its rail; content stays usable but is not a design target.
Existing 1280px checks can remain inexpensive compatibility coverage.

Mobile, small-screen optimization and exhaustive responsive matrices are low
priority. Extra keyboard interactions, shortcuts, focus polish and keyboard-only
acceptance are optional and must not hold up functional delivery. Keep existing
native control semantics and working keyboard behavior; do not spend further
time expanding or perfecting them unless the user asks. This supersedes earlier
mandatory screen-width/keyboard proof wording in desktop plans and active packs.
Core behavior, data safety and permissions remain required.

### Routes, identity and state

Global navigation lists Brains `/`, My agents `/agents` and, for the
installation owner, Connectors `/connectors`. Team `/team` is a normal,
persistent bottom sidebar item for the same installation owner, never hidden in
an account disclosure. Global navigation stays available inside a Brain,
directly above the Brain selector and the Brain's own navigation; the Brain
switcher and All Brains link replace a second full global menu. Runtime
diagnostics is an always visible, labelled top-right icon on every route,
including when healthy; diagnostic detail keeps existing operator authority.
Access tokens is a visible account-scoped entry with credential metadata and a
one-time-secret lifecycle governed by the
[direct-auth contract](mcp-plugin-direct-auth.md).

The installation library retains the last actually selected accessible Brain as
navigation context, without choosing an arbitrary Brain or scoping the global
catalogue to it. Resolve that identity against the current authorized Brain
list; hide it on revoked access, archive, failed refresh or account/session
change. Store only a transient navigation identifier, never protected Brain
payloads.

Inside `/brains/{brain}`, the contextual sidebar groups its destinations into
four ordered tiers: Brain, holding Dashboard and Ask; Knowledge, holding Memory,
Sources, Graph and Repositories; Wiring, holding Agents, Connections and
Settings; and Assurance, holding Activity. Their suffixes are `/dashboard`,
`/ask`, `/memory`, `/sources`, `/graph`, `/repositories`, `/agents`,
`/connections`, `/settings` and `/activity`. Every suffix remains a real route;
the Knowledge views share one surface and one inspector region rather than four
peer pages, as the [tiered surface contract](desktop-knowledge-surface.md)
specifies. The contextual sidebar is the sole navigation between the four
Knowledge destinations: those pages do not repeat them as a horizontal tab
strip. Pages whose tabs name sub-sections that are absent from the sidebar
(Agents, Activity, Connections, Settings) retain their intra-page tabs.
`/brains/{brain}` resolves to Dashboard with the real recorded pipeline; Ask
opens its question composer; Search evidence remains the explicit alternative
mode and stays functional when answering is disabled or unavailable.

Every destination has a real route and usable Back/forward, reload and direct
link behavior. Validated URL state holds non-sensitive tabs/filters and exact
same-Brain identifiers. Unknown section/tab values fall back to the documented
default; foreign identifiers fail through canonical reads. Questions, answers,
retrieved bodies, private checkout paths, secrets and tokens never enter URLs.
Resource inspection preserves exact revisions and the return location.

Mount only the current route's feature queries/polling, shared shell metadata
and mandatory checks for currently displayed protected content. Lazy-load feature
bundles; other presentation loading may be deferred, while authority refresh,
expiry gates and exact inspection remain intact. Brain/selection changes cancel
superseded requests and clear incompatible state; late responses cannot replace
the new selection. Logout, current access failure, observed epoch change and
advertised expiry clear affected payloads. Do not retain old protected data next
to a failed refresh. Existing bounded validity polling remains until an equally
strong replacement is proven.

### Shared visual language

[ADR 0025](../adr/0025-two-theme-desktop-redesign.md) governs this section; exact
values and component specifications live in the
[design system](../foundation/design-system.md). The two-theme migration is
presentation only. It changes no route, URL state, API call, polling cadence,
permission, policy, retention rule or failure state, starts no job and calls no
provider. Delivery is recorded in the
[desktop visual experience epic](../roadmap/epics/desktop-visual-experience.md),
where the redesign shipped on 2026-10-20.

Approved concept images and their visual wording (cream or paper surfaces, sage
headers, serif titles, traffic-light chrome, a 248px shell) govern composition,
hierarchy and placement only; colour, type, radius, depth, theme and shell
geometry follow the current design system. The six management concepts govern
composition and hierarchy, including header-aligned inspectors, compact
horizontal controls, retention-first Privacy and separate AI models/options
cards; existing brand assets, authorized records and policy semantics remain
authoritative. Their invented names, labels, counts,
dates, identities, commands, logos and generated icon variations are
illustrative, not API data: reuse the existing brand assets and real authorized
data. A redesigned page keeps every control, state, URL and permission this
contract requires.

Use the accepted primitive, semantic and component token layers, expressed as
OKLCH CSS variables under shadcn's names plus Recollect's `brand`, status,
`kind-*`, terminal and ANSI roles. Feature code uses only token-backed Tailwind
utilities or variables: no raw colour, arbitrary colour value, default-palette
utility or inline style colour outside the design layer. Tailwind CSS v4 and
shadcn/ui on one primitive family replace Mantine; no Mantine component or
generated Mantine class remains once the migration ships.

Two themes, Light and Ink, are complete and equally supported. The theme follows
the operating-system setting until a person chooses one; an explicit choice is
remembered as a non-sensitive browser preference, never as a Brain or account
policy, and is applied before first paint so no page flashes the other theme.
Every surface, chart, graph, diagram, status and focus state is defined in both
themes, and both meet the contrast, focus and non-colour status rules below.
Jade `brand` marks only AI and learning states: Ask, learning, live processing,
checked controls, focus and the active navigation icon. Primary actions use ink
`primary`, never jade. Status always pairs its colour with a word and its icon.

Mona Sans sets the interface: page titles 28/34 600, section titles 20/28, card
and dialog titles 16/24, body 14/20, labels 13/18 and captions 12/16. JetBrains
Mono sets machine text — identifiers, times, durations, counts and code — with
tabular figures. Newsreader is used only for Ask answers and quoted memory
(17/28), the Ask hero (40/46) and the wordmark; it never sets interface chrome.
Self-host the three families with pinned manifest hashes and OFL notices; no
runtime external font request. Lucide icons use one shared size/stroke
convention (16px, 1.75 stroke) and navigation map rather than per-page variants.

The Brain shell places the sidebar (240px, or a 56px rail when collapsed)
directly on the canvas and renders content in one inset main panel with a sticky
topbar. All routes share one shell scale: route-specific management styles must
not enlarge the sidebar, controls, typography or content gutters. Readable pages
cap at 1200px and tables at 1600px; Graph and pipeline canvases are full-bleed
and controls never stack above a canvas. Page headers are compact sans headers
with at most two actions; only Ask uses the serif hero above its composer. No
page uses an eyebrow.

Selecting a record opens it in the one shared inspector, docked at 400px inside
the main panel from 1440px and opened as a sheet with a scrim below that, instead
of an overlay drawer. Only one inspector is open at a time. Its selection lives
in validated URL state with the exact same-Brain identifier and revision; closing
it returns focus to the row that opened it; and a docked inspector never covers
the graph canvas. Other existing drawers that are not record inspection (Graph
filters, insights, status and paths; Agents Manage access) keep their content,
actions, permissions and URL state and are presented as sheets. Agents, Team,
Runtime diagnostics, creation and identity dialogs use the same shared panels
and context-preserving sheets.

A command palette opens with ⌘K (Ctrl+K elsewhere) from any authenticated page.
It searches memory and sources through the existing authorized
[literal list search](#literal-list-search), jumps to any route the account may
open, offers non-destructive actions, and shows at most eight rows per group. Its
first row hands the typed query to Ask, which ⌘↵ also does; the hand-off is an
explicit Ask request under the [answer contract](retrieval-answers.md) and keeps
the question out of URLs. Typing in the palette never calls a model, and
superseded requests are cancelled so a late response cannot replace newer
results. Destructive actions never run from the palette.

Motion is purposeful and fluid: shared duration/easing tokens (instant 100ms,
fast 160ms, base 220ms, slow 360ms, ambient 1600ms; standard, out, in and small
bounce easings) with staggered first-load entrance for lists (30ms steps, at most
ten), live status, shaped skeleton loaders and page-content transitions. Loops
mean live work: a Brain with nothing happening is still. Ask shows a shimmer line
while gathering evidence and reveals a validated answer whole, never streaming
unvalidated tokens. Animations never change what data is shown, never imply
precision the feeds do not have, and honor `prefers-reduced-motion` by degrading
to static content. No decorative 3D rendering. Motion handles shared
layout/transitions and React Flow bounded read-only diagrams; CSS/SVG remain
appropriate for simple effects. The knowledge graph keeps its
[ADR 0021](../adr/0021-grouped-spatial-graph-navigation.md) renderers, themed
from the same tokens in both themes. Load diagram dependencies only on their
consuming routes.

Agent and tool output renders in a terminal card on a fixed dark terminal surface
in both themes, with a bar naming the host, command and result state instead of
window traffic lights: monospace body, bounded safe ANSI subset (colors/bold/dim;
no cursor escapes) rendered as text, sanitized input, and the same
truncation/redaction as the underlying detail views. The Rust plugin CLI presents
an animated startup banner and live status lines (spinner, colored outcomes,
elapsed time) on a TTY, falling back to plain output for non-TTY, `NO_COLOR` or CI.

Create an original vector Recollect mark expressing remembered fragments gathering
together, plus a reusable wordmark/compact icon and SVG favicon. It must remain
recognizable at navigation size and have appropriate text alternatives or
decorative semantics. In the product the mark sits on its fixed paper app-icon
tile so its original colours read in both themes; the wordmark is Newsreader with
a jade full stop. A raster image is not a substitute.

The design system's shared components (among them AppShell, Sidebar, Topbar,
PageHeader, Inspector, CommandPalette, AskComposer, AnswerCard, StatusBadge,
AsyncState, Table, TerminalCard and PipelineFlow) and the FilterBar,
ScopeSummary, SettingsSection and ActionMenu compositions built from them own
recurring behavior. Keep forms, dialogs, tables and graphs on that system. No
mobile redesign. Apply the display and interaction priority above. Keep readable
long labels and non-color status meanings. Auth, invitation, OIDC error/recovery
and expired-session views use the same system in both themes; the expired-session
view is the sign-in view with a
"Your session ended — sign in again" notice, shown only after
`GET /api/auth/me` confirms the session ended and never after an explicit sign-out.

### Page responsibilities

| Destination | Required behavior and boundary |
| --- | --- |
| Ask | Question/citations/inspector governed by [answers](retrieval-answers.md); an empty thread opens on the serif Ask hero above the composer; the default view is a chat-style conversation thread — user questions and model-grounded answers as turns, with citations as expandable evidence attachments and follow-ups in the same thread — and Search evidence is an explicit alternative mode retaining Matches, Disagreements, Sources, Copy context and advanced exact/mode/time/budget criteria. The thread stays temporary; applied restrictions stay visible. |
| Memory | Readable claim/decision/procedure/handover list and exact detail/history/support. Optional correction, review, withdrawal, conflict resolution and erasure retain canonical handlers. Handover generation selects exact claim revisions, never silently saves Ask text. |
| Sources | Current paste/text-file/reference import, list/version content, collections/areas/environment filters and one Manage views editor. New-version editing and contextual reprocess/learning/erase retain their effects. Show reference-only/unavailable bytes honestly. Storage policy lives in Settings. |
| Graph | Canvas first, compact kind/scope toolbar, filters/insights/status drawers and canonical inspector. Secondary Graph tools sit behind one menu without hiding applied scope, coverage or failures. Provide keyboard/list selection and textual paths. Default bounded knowledge overview uses current Brain-only investigation selection when no explicit selection exists; it invokes the existing read endpoint once, without model/rebuild/analytics. Explicit historical/repository/manifest selections are never guessed or reset. Missing exact inputs ask for selection. Existing bounds and oversize refusal remain. |
| Repositories | Published repositories, snapshots, files/facts/coverage/contributors/insights/receipt; Environments tab owns exact revision manifests/history. Browser publishing opens accurate native instructions. Own checkout paths remain account/device-private. Committed, desired and observed revisions stay distinct. |
| Agents | Sole home for connecting a coding tool that acts as the signed-in user. Normal Agents is an observed roster beside a compact selected-agent inspector, followed by Connect, concise host setup and a real memory-read check; it has no session or context tabs. The roster shows agents across users observed on this accessible Brain, grouped by user ("mike — N agents"), with host icons; each row is one connection of exactly two kinds — Recollect plugin or direct MCP access token — with host kind, credential state, last used on this Brain, and an own-credential-only revoke action labeled as removing the keycard from all Brains; revoked/expired rows hide behind an explicit toggle. Only real observed use associates an agent with a Brain; safe identity/host/last-use metadata spans contributing users, while private tasks, payloads, foreign credential dates and revocation authority are not shared. The inspector's activity groups bounded captured events by date with title, time and one recorded outcome; evidence and technical detail expand per event. Capture receipts are events with exact evidence, not a synthetic session transcript. Credential enabled is not online, recorded capture is not learned knowledge, and partial coverage remains qualified. Private contexts, own task/scope and local folders are managed automatically, with no normal navigation, helpers or drawers; detailed verification is troubleshooting; legacy contexts links normalize to Agents, and APIs, discovery and privacy isolation stay intact. A link opens the global Agents page for the complete account list with per-Brain usage; there is no redundant My agents footer. Setup is not an active connection; its explicit link to recorded Brain activity never infers success from another agent's activity. Capture policy has one editor in Settings. Browser metadata retry and native discovery are distinct. Other accounts' tasks remain private even from Brain admins. |
| Connections | Outbound Brain-managed MCP only; no agent-connection setup path. Connections/Tool access/Private Runners tabs as specified under [Connections management](#connections-management). Independent Use/Manage/Share grants remain. No arbitrary executable installation. |
| Activity | Exception-first drill-down behind the standing assurance band, which renders only while something needs attention — a blocker, pending read, failed feed, archived Brain, disabled autonomous memory, or an empty first session — and stays hidden on a healthy Brain. Timeline/Processing/Tool calls/Model usage compose existing bounded authorized feeds, with canonical detail links, and host the [live pipeline](#live-pipeline-and-ambient-view). No fabricated counts or total causal ordering. Admin audit, profile outputs and private contexts retain independent authorization. |
| Agents (global) | Roster at `/agents`: every user, including installation owners, sees only their own agents. Group compact rows by accessible Brain with observed use; omit empty Brain groups, while unused credentials remain reachable in Manage access. Show name, host/integration and last use there; entry point from every per-Brain roster. Pairing approval and complete own-account credentials/history live in the Agents Manage access drawer. Access tokens uses searchable compact rows, concise host/integration/status, last-use and expiry columns, on-demand identifiers and the existing confirmed own-account revoke action; historical records remain filterable. `/devices` remains a compatibility redirect, preserving pairing codes. |
| Settings | General/Access/Privacy/AI permissions, with one editor per policy, valid custom restrictions and rare repair/history actions secondary; see [Settings](#settings). General owns archive/reopen and irreversible Brain deletion through the preview and confirmation flow in [brain deletion](platform-brain-deletion.md), kept visibly distinct from archive. Permission, content classes, purposes and quotas are editable by actual authorized roles. Capture, model transmission, answering and retention remain separate decisions. |
| Brains | Chooser for create/open, personal/shared, archived and search views with supported metadata; cards open a Brain directly. It has no global pipeline or activity preview; processing stays in per-Brain Activity. Installation health is a small affordance with owner diagnostics. |
| Team | Existing account/invitation/local/OIDC/disable/reset/audit functions, installation-owner only; Brain access remains a different resource. |
| Devices compatibility | `/devices` redirects into Agents Manage access; `/devices?code=…` opens pairing there, including across sign-in. No separate normal Devices page. Pending approval/deny/revoke, history and own-account boundaries remain. |

Normal autonomous learning, capture and agent scope/tool work require no new
mandatory human clicking. Existing capability controls remain reachable in their
owning page or contextual inspector. Hiding a control does not change its default
or permission. Counts/statuses need actual response support; unavailable data is
not replaced with sample/mock values.

### Connections management

Connections uses Connections/Tool access/Private Runners tabs, searchable compact
rows and one selected inspector with Overview/Tools/Activity. Search filters only authorized loaded
connections and distinguishes no matches from no setup. Connections keeps Tool
access and explicit execution placement; runner/schema/test details are
secondary. Private-network execution is secondary and clearly separate from
ordinary plugin memory access. Configuration and past successful calls remain
distinct from present health: last successful call is historical, separate from
a current check. Rows carry compact badges for the connection's actual
Brain/environment scope and its central/device/private-runner placement.

Setup is provider-neutral, with no Context7 shortcut. It follows approved
connector → target/environment → credential reference → runner → review →
explicit test, presented as one wide form offering Server URL, Paste config or
Approved connector, with explicit review and optional bounded metadata
inspection. New HTTP server addresses are the primary owner path; the
installation's approved connector catalogue is a named alternative with real
connector names/counts. JSON, TOML and YAML host MCP configs are parsed without
executing commands; multiple entries require selecting one server. Non-secret
fields are preserved, credential values move to masked controls, and unsupported
routing is rejected. Installation owners manage reusable connector definitions
outside Brains through an import/review/approve flow; approval itself makes no
provider or tool call.

Metadata inspection uses the existing owner-only anonymous HTTP handshake and
tool-list endpoint. Test connection is explicit and separate from saving or
executing tools: it performs only that bounded anonymous handshake and tool
metadata check, reports only that check's result and never runs a tool,
approves metadata, changes grants, or claims tool-call success, execution
permission, credentials, persistent running state or future health. The setup
wizard's explicit test may have effects and never implicitly retries uncertain
work. Authenticated/local/private setups direct users to an authorized actual
tool call; no arbitrary automatic tool payload or execution is introduced. Schema
compatibility failures explain supported limits.

The connection preview is editable in its existing inspector, retaining identity,
tabs, section positions and reserved Save/Cancel controls. The read-only
inspector has no More configuration section; existing editable fields and
credential authority remain available in Edit.

Cached tool lists have concise summaries, optional full descriptions and schemas,
and preserve independent Use/Manage/Share grants. Approved tool reads show
labelled loading, empty, error/retry and permission states; a pending, failed or
disabled read is never displayed as zero tools. Read retries are bounded and do
not invoke tools or start dormant connections. A tool description appears once
per expanded view, with the compact summary hidden while the full description is
open.

The call inspector groups actual service/group/scope/runner/status/timing
metadata into labelled rows and cards, with structured result and evidence
sections. Existing Use/ownership, expiry, unknown completion, cancellation and
reconciliation gates remain.

#### Rendered metadata and code

Reuse the bundled safe Markdown/code renderer for JSON/TOML/YAML and fenced
blocks throughout management, with the installed lazy syntax highlighter rather
than executable grammars fetched from remote servers. Tool descriptions use
readable Markdown paragraphs, lists and preserved source line breaks. For legacy
single-line metadata, presentation may restore explicit collapsed list/numbered
boundaries without adding, deleting or reordering source words; vendor prose
indentation may be normalized for display only. Original metadata stays
unchanged, and genuine fenced/indented code and inline/code-block bytes stay
exact. Remote content stays inert, without external images or executable HTML.
Do not fetch or update an approved external connector just to format it.

JSON blocks default to readable pretty formatting and offer explicit
Format/Compact controls; invalid or precision-unsafe JSON remains exact. Copy
retains exact source bytes; for these JSON blocks Copy uses the displayed
representation unless an explicit secret-safe copy payload already exists. No
jq query interpreter or new code execution is introduced. Optional connector
icons follow the amended catalogue trust and resource bounds.

#### Tool access

Tool groups use one bounded card per real group with an identity header, icon
tile and title in the design system's styling, MCPs and people inside the card,
and editing in place. The tab has a compact environment filter. Each card has a
compact title/Enabled header with separate environment and MCP-count badges
beneath the title, an optional description editor and thin MCP/People column
separation. Retain the existing MCP picker and canonical configuration/grant
save flow. Read-mode cards use the same compact scale without duplicated
headings or access footers. The environment editor replaces its badge in that
same metadata row; it does not move into the right-hand Enabled/action area when
editing.

Each person has one adjacent set of independent Use, Manage and Share icon
controls beside their name: green background for granted, red for not granted,
with distinct icons, accessible names, tooltip labels and checked/pressed state.
Do not depend on colour alone. Inherited powers stay green and locked, visibly
distinct from editable direct grants; the small source chip opens concise
provenance and direct-grant editing only on demand. Draft effective rights
include pending direct and group edits without changing the server's additive
authority. Manage edits configuration; Share edits grants. Combined saves report
partial outcomes and retry only unfinished mutations unless an atomic command is
provided. Existing Share/Manage, archive, stale revision, failed refresh and
partial-save fences remain.

#### Private Runners

The tab is labeled **Private Runners**, retaining `tab=runners` and existing
registration/permission behavior. Its page heading, tabs and runner cards align
to the same left edge as Connections/Tool access, using the shared bounded left
alignment, with the 1200px content cap, no centering of the route and a
consistent gap below the tabs.

Bounded cards have a compact count/Add toolbar, the paired device under the
runner name, compact device/status badges, separate status/Enabled/Edit,
assigned connection rows with actual scope and approved tool chips, linked Tool
groups and a slim paired-devices/setup footer. Add and Edit are inline with
Enabled and stable Cancel/Save actions; the connection section stays visible
while editing an existing registration. The mockup's Enabled toggle is the
existing draft toggle in Edit; read mode retains its status without immediate
writes.

Device selection is available only during creation, using the administrator's
own active paired devices; an existing binding is locked. Device names come only
from the current user's device inventory, with a UUID fallback for other hosts.
Registration alone implies neither connectivity nor permissions or machine
location. Copy setup command is explicit and never starts a runner. Keep readers
read-only, archived Brains locked, failed reads gated and original-revision
conflicts visible without rebasing drafts. Creation requires a nonempty name and
an eligible selected device. Pending saves freeze fields and actions; failures
retain drafts and command-idempotent retry. Preserve the original revision,
pending and save/retry guards.

Expose only a canonical private runner UUID on already-authorized catalogue
connection summaries, preserving connection/profile filtering and admin-only
configuration reads. Administrators read approved definitions; other users use
existing Use-gated cached discovery with exact group environment and all bounded
pages. Labels/counts describe visible assignments/approved metadata, not
execution permission or live tools. Reader-empty says no connections are visible
to them. Share metadata reads by definition or profile/revision/environment so
rows in the same group do not repeat the full discovery traversal. Preserve
loading, empty, error/retry, denied, disabled/archive and revocation states;
failed refresh hides stale relationships/tools. A missing binding from an older
server is not shown as zero assigned connections. UUID-only links open the exact
connection Tools inspector or tool-group card. Never execute a tool, start a
runner, load credentials or fetch admin detail to render these cards.

### Settings

Settings has General, Access, Privacy and AI permissions, with one editor per
policy.

**General** is a bounded two-column view: Brain details and compact environment
rows in the main column; status/archive and clearly separated permanent deletion
in the side column. Use existing Brain name/description/icon edit semantics, with
stable inline editing where practical, and keep the canonical deletion
preview/name confirmation. Optional environments are created or renamed here
using canonical evidence groups; Brain-wide work needs none. Add/rename
environment uses an inline row within its existing list with explicit
Save/Cancel, retaining description and existing canonical evidence-group
commands. Environment selection scopes knowledge/tools and grants no access or
execution rights. Show real names/status/date only. Preserve
read-only/admin/archive, pending/error and stale-record behavior.

**Access** is a member list with add/edit/group/ownership popups. Effective roles
and grant sources remain visible on demand; removing a direct grant does not
promise removal of owner/group access.

#### Privacy

All existing capture, retention and storage fields share one inline form on the
same page, with title separators and one Edit/Save/Cancel group; there are no
separate Privacy cards, per-section editing or second privacy drawer/tab
hierarchy. Privacy is retention-first and keeps its section headings, rows and
values while editing, with inline controls in the same value column. Capture
limits and activity/backup fields are visible sections, not collapsible fields;
capture text limits are actual inline inputs under the existing Edit/Save/Cancel
and admin/revision boundary. Keep the actual day-value fields; the recovery
adapter already consumes backup windows, without a claim that a schedule is
configured or a backup exists. There is no bottom AI transmission link and no
Activity/backup explanation paragraph. Direct Privacy customization links select
an existing editor without changing policy.

The content caps at 1120px, aligned with its page header and tabs on wide
desktops. Fine row separators and a subtle focus-within highlight make each
edited label/control pair clear. Manual Excluded tools/content fields are hidden
in the normal Privacy form, including Edit: they are custom capture rules, not
automatic secret redaction. Preserve their saved arrays exactly in all canonical
saves; existing exclusions stay enforced, backend redaction and enforcement
remain, automatic secret redaction stays enabled, and presentation never
discards excluded values.

One UI Save uses the four existing canonical commands. Send only changed domains,
keep capture/retention revision and idempotency boundaries, preserve storage
drift checks, and acknowledge each successful domain before continuing. Boolean
storage editors reject changes observed since opening but do not claim atomic
revision fencing from the existing boolean-only API. If a later command fails,
retain remaining drafts and identify already saved domains; retry skips
acknowledged work. Cancel discards remaining drafts without undoing successful
commands. No atomic cross-policy transaction is implied. Read errors, archive and
authority loss disable editing; saved exclusions and all untouched policy fields
stay intact.

Existing stored policies, exclusions, source permissions, class deadlines and
distinct storage policies are preserved. Defaults stay 30 days for raw
sessions/tool outputs, durable evidence/memory until erased, automatic redaction,
and repository file text disabled unless explicitly permitted. Edits are optional
exceptions; shortening retention keeps its existing immediate-expiry warning.

#### AI permissions

AI permissions keeps the accepted layout with inline purpose/content/automatic
controls and explicit Cancel/Save. It displays allowed processing/content,
installed provider/model identity with the provider logo aligned to the provider
name, and truthful search exceptions. Automatic processing controls are visible
without disclosure. There is no Privacy reminder, no Search coverage/review card
and no recommended-default control. Diagnostic and history feeds load only when
opened; optional diagnostics retain actual recorded outcomes. The existing
autonomous semantic contract governs bounded recovery and terminal idle failures;
ordinary use does not require a person to review failed inputs.

Model selection happens in the model cards; Selected models and More options sit
in the right sidebar. Prices have dated official provenance and
unavailable/unknown states. An embedding change explicitly saves and rebuilds
model/dimension-isolated representations under standing policy; lexical access
remains available. Managed defaults apply once on Brain creation, preserving
existing customized policies. Provider and semantic contracts govern backend
selection, transmission and generation changes.

Processing limits sits in the main form below Automatic processing, with the
four existing fields: maximum output tokens, maximum input bytes, daily token
allowance and concurrent model calls. Use a compact two-column grid that becomes
one column on narrow screens, with the same section separator and the shared
Edit/Save/Cancel policy command. Values, bounds and authority remain canonical.
Its explanation sits behind a keyboard-accessible info icon beside the heading,
with no permanent helper paragraph.

Autonomous learning/acceptance is the normal managed path. Legacy literal-only
acceptance and explicit learning controls appear only when autonomy is disabled;
they are not extra requirements for managed learning. Provider transmission
permission remains explicit and separate from capture/tool access. Existing
custom policies are never silently converted.

### Brain artwork

Brain creation/editing supports PNG, SVG and ICO artwork up to 512 KiB. Browser
image-context rendering converts artwork to at most 256px PNG; originals are
never inserted as executable markup or retained. The server independently checks
PNG format, dimensions and decoder allocation, then normalizes the bytes. Store
only bounded PNG bytes and revision on the Brain row, never in receipts or audit.
Reading follows Brain visibility; changing/removing follows Brain admin and
browser CSRF authority. Responses are authenticated, no-store and nosniff.
Whole-Brain deletion and restoration fencing cover the icon through the Brain row.

### Live pipeline and ambient view

Activity shows actual current or last-observed activity, including multiple
agents and the information being processed. Its live pipeline view — capture →
learning → memory/graph stage nodes with animated edges — animates state diffs
over the existing polling feeds: stages stay in place while their state changes,
new items slide in, the edge feeding a running stage flows, counts tick, and a
processing stage glows while a learning job runs. One edge packet represents one
recorded event; no events means no packets. It shows only real feed data: no
fabricated counts, streaming text or causal ordering, and its cadence remains
the contracted feed refresh below.

`GET /api/brains/{brain}/pipeline` is a bounded read projection of existing
capture, source, job and learning records, not a second event store. Return up to
30 entries, active work first then most recently changed, with `has_more`, a
server observation time and a display validity deadline of at most six seconds.
Join capture to its exact source version; source processing to the job targeting
that version; learning to its exact source version and job; memory outcomes to
that run. Never join independent counters or adjacent timestamps into a causal
chain. A successful run may produce zero memories. Outcome counts describe the
recorded run, not the current accepted memory inventory. Graph generation is a
separate Brain-wide projection and must not animate as that source's proven
output. The endpoint cannot enqueue work, call a model or change policy.

Published contributor identity comes from the capture binding/source version;
background learning is Recollect processing, even when the standing policy actor
differs. Preserve event, binding, device and reported subagent distinctions.
Missing origin information is unknown; manual inputs and managed MCP captures
have explicit labels. Credential enabled means unrevoked/unexpired, not online
or running. Global agent metadata is own-account-only, including for installation
owners. Within an accessible Brain, observed agent identity and last use may
span users, without sharing foreign credential timestamps or revocation
authority. Private tasks and unshared prompts remain outside all these views.

Recent inputs distinguish source processing failures from learning failures.
Budget, input size, sensitive input and model output failures use explicit
labels and an on-demand reason; capture-only events remain capture-only. Do not
infer successful learning, rewrite historical outcomes or override policy caps.

The selected feed refreshes every two seconds while visible. Label the cadence
and last check; do not imply token streaming or stages missed between polls.
Show currently queued/running work distinctly from recent outcomes. Initial
history is static; stable ID/state changes may produce one short path pulse.
No demonstration/replay data or timer-generated activity enters product views.
Failed, expired or forbidden refreshes clear protected payload and halt motion;
selection changes cancel requests and reset the animation baseline. Ordinary
background refreshes and brief visibility changes keep an unexpired, successful
snapshot mounted, with motion paused while hidden. A visibility event does not
renew its deadline. Brain metadata revisions refresh assurance reads without
remounting the route; a changed Brain or effective role still resets its
boundary. Re-check effective retention/erasure on every read, omit removed
content and never expose private task context. Pause hidden-page motion and
honor reduced motion.

A fullscreen ambient mode at `/brains/{brain}/tv` (hidden from navigation,
toggled from Activity) cycles the Brain's recent memories one at a time with slow
fade/scale transitions, category glow, timestamp and provenance in the current
theme. It reads only the existing authorized bounded claim page. That page
advertises its earliest canonical retention deadline as optional `expires_at`,
computed from the displayed claim revisions and supporting evidence whose
retained labels were emitted. `knowledge_until` remains historical version
closure, not retention. TV clears every displayed card and feed timestamp on
error, advertised expiry or access loss, and re-reads on re-entry. It tracks
exact revision identities across list insertions, presents review, freshness,
operational and conflict/rule qualifications, pauses while hidden, honors
reduced motion, and provides an explicit exit with isolated focus.

### Literal list search

Extend authorized source and claim list endpoints with optional `q`, trimmed,
at most 200 UTF-8 bytes. Empty means the existing unfiltered list.
Search is case-insensitive literal containment: `%`, `_` and escapes are ordinary
characters. Source search covers the current source title; Memory search covers
displayed assertion fields: subject, predicate, value and rationale.
The UI labels this scope. This first list search does not scan raw retained source
bodies, use a model, replace Recall or imply exhaustive evidence retrieval.

Apply current Brain, role, lifecycle and existing list filters first and apply
`q` before LIMIT/OFFSET. Preserve stable existing ordering and pagination. No
match is distinct from an empty Brain. Changing search or filters resets offset;
the browser consumes cancellation and stale pages cannot appear under a new query.
Existing exact detail, historical inspection and read/erase fences are unchanged.

### Page recovery

HTML entry responses require revalidation; missing hashed assets return 404
rather than the HTML fallback. A failed lazy page shows a styled recovery panel
with a user-triggered Reload page action preserving the current URL. Never
automatically reload or discard an open editor, loop retries, or display raw
module URLs and stack traces. Other page errors retain a neutral recovery state.

### Permissions and failure behavior

Disabled answering leads to exact/text search, not a fake answer. Configured
credentials/models/agents/tools are not reported connected without a successful
corresponding call. Explicit model/tool checks show cost/effect implications.
Never infer profile Use from Brain membership or installation ownership. Keep
all existing real actions, access boundaries and source/model disclosures.
Existing private records, grants, retention granularity, deletion fences,
real-call semantics and safe deep links remain.

Every page handles loading, empty, partial, failed, forbidden, archived and stale
states. Preserve safe unsaved input on recoverable errors, but clear invalid
protected payload. Destructive or effectful actions retain actual previews,
permissions, version checks and uncertainty. This redesign does not reset data,
enable model transmission or change capture/retention policy during migration.
No code is partitioned by file length; existing files retain their roles, and
no file-size refactor or new dependency is required for this polish.

## Acceptance

Prove all twelve destinations with real route navigation, direct links, Back/
forward, deep detail and role-appropriate actions. Focused backend tests prove
search before pagination, literal special characters, foreign Brain isolation,
empty/limit behavior and no model call. Existing mutation/domain regressions
must pass from their new locations. Graph proof covers an automatic bounded read,
no hidden expensive action, keyboard inspection and invalidation. Test private
tasks, profile grants, disabled/unconfigured states, expiry and in-flight Brain
switches with positive useful controls. Measure request volume/performance rather
than claiming improvement by layout alone. Verify the actual desktop assets,
fonts/logo and browser behavior; run frontend checks and `./scripts/validate.sh`.

Pages built from an approved concept image (the management concepts, compact
tool-access cards and Private Runner cards) need actual rendered comparison with
that image and independent before/vision/runtime reinspection, as the
[fidelity follow-up](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/desktop-management-vision-fidelity.md)
requires. Tool access proves Save/Cancel; Private Runners compares actual read
and Edit at laptop and wide sizes. No saved policy change is needed for visual
proof of the Privacy form.

The two-theme redesign additionally proves, in a real browser in both Light and
Ink at the review sizes above: every destination renders with token-backed
styling and no Mantine dependency; the theme follows the system setting, keeps an
explicit choice across reloads and paints no frame in the other theme; the
inspector docks at 400px from 1440px and opens as a sheet below it, with focus
return and URL state; ⌘K opens the palette, its results respect authorization,
it hands a query to Ask without putting it in the URL and runs no destructive
action; only Ask shows the serif hero; reduced motion shows complete static
content; and the updated design check rejects a raw or default-palette colour.
Existing route, Playwright and domain regressions pass unchanged in meaning.

## Explicit Deferrals

Mobile/small-tablet redesign (below 1440px only the sheet inspector and sidebar
rail are specified), arbitrary MCP executable approval,
browser provider installation, broader administrative MCP parity, new source
formats/connectors and general-purpose agent hosting. Saved or action-capable
Ask requires separate authority. No external deployment, automatic version bump,
commit or push is implied by this desktop delivery.
