# Contextual desktop experience

Status: accepted

## Connection readability follow-up — 2026-10-06

The user retains the Connections layout and asks for compact badges showing
the connection's actual Brain/environment scope and central/device/private-runner
placement. Last successful call is historical, separate from a current check.
Approved tool reads show labelled loading, empty, error/retry and permission
states; a pending/failed/disabled read must never be displayed as zero tools.
Read retries are bounded and do not invoke tools or start dormant connections.

Tool descriptions appear once per expanded view, with the compact summary hidden
while the full description is open. Vendor prose indentation may be normalized
for display only; genuine fenced/indented code stays exact. Retain safe Markdown
and the installed lazy syntax highlighter rather than fetch executable grammars
from remote servers. JSON blocks default to readable pretty formatting and offer
explicit Format/Compact controls; invalid or precision-unsafe JSON remains exact.
Copy uses the displayed representation unless an explicit secret-safe copy payload
already exists. No jq query interpreter or new code execution is introduced.

The call inspector groups actual service/group/scope/runner/status/timing metadata
into labelled rows and cards, with structured result and evidence sections. Existing
Use/ownership, expiry, unknown completion, cancellation and reconciliation gates
remain. Test connection performs only the explicit bounded anonymous handshake and
tool metadata check; it never runs a tool, approves metadata, changes grants or
promises future health. Schema compatibility failures explain supported limits.

## Bounded Privacy rows follow-up — 2026-10-06

The user's latest feedback caps the Privacy content at 1120px, aligned with its
page header and tabs on wide desktops. Use fine row separators and a subtle
focus-within highlight to make each edited label/control pair clear. Hide manual
Excluded tools/content fields in the normal Privacy form, including Edit. These
are custom capture rules, not automatic secret redaction. Preserve their saved
arrays exactly in all canonical saves; backend redaction and enforcement remain.
The user selected the generated General and inline environment layouts in
`output/imagegen/2026-10-06-settings-refinement`. Implement a bounded two-column
General view: Brain details and compact environment rows in the main column,
status/archive and clearly separated permanent deletion in the side column.
Use existing Brain name/description/icon edit semantics, with stable inline
editing where practical. Keep the canonical deletion preview/name confirmation.
Add/rename environment uses an inline row within its existing list with explicit
Save/Cancel; retain description and existing canonical evidence-group commands.
Show real names/status/date only; mockup dates/counts/identities are illustrative.
Preserve read-only/admin/archive, pending/error and stale-record behavior.

## Unified Privacy form follow-up — 2026-10-06

The user's latest correction supersedes separate Privacy cards and per-section
editing. All existing capture, retention and storage fields share one form, title
separators and one Edit/Save/Cancel group. Remove the bottom AI transmission link
and the Activity/backup explanation paragraphs. Keep the actual day-value fields;
the recovery adapter already consumes backup windows, without a claim that a
schedule is configured or a backup exists.

One UI Save still uses the four existing canonical commands. Send only changed
domains, keep capture/retention revision and idempotency boundaries, preserve
storage drift checks, and acknowledge each successful domain before continuing.
If a later command fails, retain remaining drafts and identify already saved
domains; retry skips acknowledged work. Cancel discards remaining drafts without
undoing successful commands. No atomic cross-policy transaction is implied.
Read errors, archive and authority loss disable editing; saved exclusions and
all untouched policy fields stay intact. Retention shortening keeps its existing
expiry warning. No saved policy change is needed for visual proof.

### Visible management settings follow-up — 2026-10-06

The user's seven marked follow-ups supersede the earlier account-menu and
coverage-card presentation below. Team is a normal, persistent bottom sidebar
navigation item for the same authorized installation owner. It is not hidden in
an account disclosure. Remove the redundant My agents footer from each Brain
roster and More configuration from the read-only connection inspector; existing
editable fields and credential authority remain available in Edit.

Tool descriptions use readable Markdown paragraphs, lists and preserved source
line breaks, with the existing bundled syntax highlighting for code. For legacy
single-line metadata, presentation may restore explicit collapsed list/numbered
boundaries without adding, deleting or reordering source words. Keep original
metadata unchanged and inline/code-block bytes exact; remote HTML/images remain
inert. Do not fetch or update an approved external connector just to format it.

AI settings omit the Privacy reminder and Search coverage/review card. Automatic
processing controls are visible without disclosure, and the provider logo aligns
with the provider name. Optional diagnostics retain actual recorded outcomes.
The existing autonomous semantic contract governs bounded recovery and terminal
idle failures; ordinary use does not require a person to review failed inputs.

Privacy's capture limits and activity/backup fields are visible sections, not
collapsible fields. Capture text limits are actual inline inputs under the
existing Edit/Save/Cancel and admin/revision boundary. Empty exclusions are not
shown in the ordinary read view; editing exposes optional exclusions and any
existing exclusions remain visible, editable and enforced. Automatic secret
redaction stays enabled and excluded values are never discarded by presentation.

### Browser feedback implementation — 2026-10-05

The user approved implementation of all ten items in the
[browser review](../research/browser-management-review-2026-10-05.md), with the
revised Connections-style tool-group cards replacing the rejected list/inspector
concept. All routes share the existing 248px shell, 14px navigation and compact
Newsreader page-heading scale; route-specific management styles must not enlarge
the sidebar, controls, typography or content gutters. Diagnostics is an always
visible, labelled top-right icon, including when healthy; Team stays in account
controls. Diagnostic detail continues to use existing operator authority.

Agents retains the observed roster beside a compact selected-agent inspector.
Activity groups bounded captured events by date and shows title, time and one
recorded outcome; evidence and technical detail expand per event. Credential
enabled is not online, recorded capture is not learned knowledge, and partial
coverage remains qualified. Private contexts and local folders are managed
automatically and have no normal navigation, helpers or drawers. Legacy contexts
links normalize to Agents; APIs, discovery and privacy isolation stay intact.
Access tokens is a visible account-scoped entry with credential metadata and
one-time-secret lifecycle governed by the direct-auth contract.

Connection preview becomes editable in its existing inspector, retaining
identity, tabs, section positions and reserved Save/Cancel controls. Testing is
explicit and separate from saving or executing tools. Tool groups use one paper
card per real group, a sage identity header, dark icon tile and serif title, with
MCPs and people inside the card and editing in place. Use, Manage and Share are
independent icon controls beside each principal's name: green background for
granted, red for not granted, with distinct icons, accessible names, tooltip
labels and checked/pressed state. Do not depend on colour alone. Effective
inherited rights stay distinguishable from editable direct grants. Manage edits
configuration; Share edits grants. Combined saves report partial outcomes and
retry only unfinished mutations unless an atomic command is provided.

Reuse the bundled safe Markdown/code renderer for JSON/TOML/YAML and fenced
blocks throughout management. Copy retains exact source bytes; remote content
stays inert, without external images or executable HTML. Optional connector
icons follow the amended catalogue trust and resource bounds.

AI settings retain the accepted layout with inline purpose/content/automatic
controls, explicit Cancel/Save and model selection in the model cards. Prices
have dated official provenance and unavailable/unknown states. An embedding
change explicitly saves and rebuilds model/dimension-isolated representations
under standing policy; lexical access remains available. Recommended-default
controls are removed. Managed defaults apply once on Brain creation, preserving
existing customized policies. Provider and semantic contracts govern backend
selection, transmission and generation changes.

### Management vision fidelity correction — 2026-10-05

After comparing before/vision/after, the user rejected the delivered visual match
and explicitly requested a UI agent to reproduce the approved styling, formatting,
icons and edit fields. The six final images govern composition and hierarchy,
including header-aligned inspectors, compact horizontal controls, retention-first
Privacy and separate AI coverage/models/options cards. Existing brand assets,
authorized records and policy semantics remain authoritative. The
installation library retains the last actually selected accessible Brain as
navigation context, without choosing an arbitrary Brain or scoping the global
catalogue to it. Resolve that identity against the current authorized Brain list;
hide it on revoked access, archive, failed refresh or account/session change.
Store only a transient navigation identifier, never protected Brain payloads.
The
[fidelity follow-up](../roadmap/execution/archive/desktop-management-vision-fidelity.md)
requires actual rendered comparison and independent reinspection.

### Approved management concepts — 2026-10-05

The user approved all six generated management concepts and requested complete
implementation followed by independent before/vision/runtime UI review. The
images in `output/imagegen/2026-10-05-management-concepts-01a10b36` establish layout
direction; their invented names, counts, commands and logos are illustrative.
Reuse the existing brand assets and real authorized data.

Global navigation remains available inside a Brain: Brains, My agents and
installation-owner Connectors at `/connectors`, followed by the Brain selector
and existing Brain navigation. Team remains reachable through account controls;
operator diagnostics uses the persistent top-right icon. Connections uses Connections/Tool access/Runners
tabs, compact rows and one selected inspector with Overview/Tools/Activity.
Configuration and past successful calls remain distinct from present health.

Connection setup presents Server URL/Paste config/Approved connector in one
wide form, with explicit review and optional bounded metadata inspection.
JSON, TOML and YAML host MCP configs can be parsed without executing commands;
multiple entries require selecting one server. Non-secret fields are preserved,
credential values move to masked controls, and unsupported routing is rejected.
Installation owners manage reusable connector definitions outside Brains through
an import/review/approve flow; approval itself makes no provider or tool call.

Privacy retains section headings, rows and values while editing. Inline controls
occupy the same value column; Cancel/Save appear within that section. Existing
capture exclusions, revision checks, retention warnings and distinct storage
policies remain. AI permissions displays allowed processing/content, installed
provider/model identity, and truthful search exceptions; diagnostic/history feeds
load only when opened. Agents displays the observed roster with host icons,
credential state and last use, followed by concise host setup and memory-check
utilities. Other users' private context and revocation rights remain protected.

The [management concept pack](../roadmap/execution/archive/desktop-management-concepts.md)
specifies delivery; the [credential amendment](mcp-runtime-and-credentials.md#browser-managed-credential-provisioning--2026-10-05)
governs secret entry independently of visual layout.

The [managed experience amendment](memory-managed-experience.md) governs the
2026-09-28 managed setup, progressive disclosure and owner connector approval
changes; earlier explicit APIs and stored policies remain compatible.

The [tiered surface amendment](desktop-knowledge-surface.md), decided under
[ADR 0017](../adr/0017-desktop-knowledge-and-ask-experience.md) on 2026-09-29,
governs sidebar tiering, the merged knowledge surface, the Dashboard landing default,
the Agents/Connections split and the assurance band. It changes placement and
presentation only: this contract's token, typography, icon, route-ownership,
desktop-width, accessibility and read-only-answer requirements remain in
force, and every route named below stays a real deep-linkable URL.

### Display and interaction priority — 2026-10-01

The user prioritizes normal laptop screens and larger desktop monitors, up to
about 32 inches. Aim for comfortable desktop reading and ordinary pointer use;
physical screen size is not a fixed CSS resolution. Use 1440×900 and 1920×1080
as practical review sizes, with wider desktop checks when a changed layout needs
them. Existing 1280px checks can remain inexpensive compatibility coverage.

Mobile, small-screen optimization and exhaustive responsive matrices are low
priority. Extra keyboard interactions, shortcuts, focus polish and keyboard-only
acceptance are optional and must not hold up functional delivery. Keep existing
native control semantics and working keyboard behavior; do not spend further
time expanding or perfecting them unless the user asks. This explicit user
decision supersedes earlier mandatory screen-width/keyboard proof wording in
desktop plans and active packs. Core behavior, data safety and permissions remain
required.

## Source

The user explicitly approved the complete desktop proposal on 2026-09-26 and
requested implementation, consistent fonts/icons/SVGs and an original SVG logo.
[ADR 0014](../adr/0014-desktop-experience-and-answers.md), the accepted
[plan](../roadmap/desktop-experience/README.md),
[design tokens](../roadmap/desktop-experience/design-system.md) and existing
[foundations](../foundation/README.md) govern this work. Concept images illustrate
layout; invented labels, counts and generated icon variations are not API data.

## Contract

### Actionable management correction — 2026-10-04

The user's annotated feedback replaces technical default forms with useful
actions. Connection setup is provider-neutral: no Context7 shortcut. New HTTP
server addresses are the primary owner path; the installation's approved
connector catalogue is a named alternative with real connector names/counts.
Metadata inspection uses the existing owner-only anonymous HTTP handshake and
tool-list endpoint. An explicit connection Test reports only that check's result,
never tool-call success, execution permission, credentials or persistent running
state. Authenticated/local/private setups direct users to an authorized actual
tool call; no arbitrary automatic tool payload or execution is introduced.
Cached tool lists have concise summaries, optional full descriptions and schemas,
and preserve independent Use/Manage/Share grants. Private-network execution is
secondary and clearly separate from ordinary plugin memory access.

Brain Access is a member list with add/edit/group/ownership popups. Effective
roles and grant sources remain visible on demand; removing a direct grant does
not promise removal of owner/group access. Privacy rows edit inline on the same
page, with Cancel/Save. Capture and retention keep their revision/idempotency
checks; boolean storage editors reject changes observed since opening but do not
claim atomic revision fencing from the existing boolean-only API. No second privacy
drawer/tab hierarchy. Existing stored policies, exclusions, source permissions
and class deadlines are preserved. Defaults stay 30 days for raw sessions/tool
outputs, durable evidence/memory until erased, automatic redaction, and
repository file text disabled unless explicitly permitted. Edits are optional
exceptions; changing retention still warns about immediate expiry.

Autonomous learning/acceptance is the normal managed path. Legacy literal-only
acceptance and explicit learning controls appear only when autonomy is disabled;
they are not extra requirements for managed learning. Provider transmission
permission remains explicit and separate from capture/tool access. Existing
custom policies are never silently converted. Optional environments can be
created/renamed in General settings using canonical evidence groups; Brain-wide
work needs none. Environment selection scopes knowledge/tools and grants no
access or execution rights. No code is partitioned by file length.

### Routes, identity and state

Global navigation lists Brains `/`, Agents `/agents` and Team `/team`; Team
remains installation-owner only. The global Agents page is the authorized agent-level
roster: every account sees only its own agents grouped by accessible Brain.
The `/devices` compatibility URL redirects into Agents Manage access, preserving
pairing codes; the complete own-account credential list lives in that drawer;
per-Brain agent visibility and labeled revocation live on the Brain's Agents
surface. Inside `/brains/{brain}`, the contextual sidebar groups
its destinations into four ordered tiers: Brain, holding Dashboard and Ask; Knowledge, holding Memory,
Sources, Graph and Repositories; Wiring, holding Agents, Connections and
Settings; and Assurance, holding Activity. Their suffixes are `/dashboard`, `/ask`, `/memory`,
`/sources`, `/graph`, `/repositories`, `/agents`, `/connections`, `/settings`
and `/activity`. Every suffix remains a real route; the Knowledge views share one
surface and one inspector region rather than four peer pages, as the
[tiered surface amendment](desktop-knowledge-surface.md) specifies. The
contextual sidebar is the sole navigation between the four Knowledge
destinations: those pages do not repeat them as a horizontal tab strip. Pages
whose tabs name sub-sections that are absent from the sidebar (Agents, Activity,
Connections, Settings) retain their intra-page tabs. The Brain
switcher and All Brains link replace a second full global menu. `/brains/{brain}`
resolves to Dashboard with the real recorded pipeline; Ask opens its question composer; Search evidence remains the explicit
alternative mode and stays functional when answering is disabled or unavailable.

Every destination has a real route and usable Back/forward, reload and direct
link behavior. Validated URL state holds non-sensitive tabs/filters and exact
same-Brain identifiers. Unknown section/tab values fall back to the documented
default; foreign identifiers fail through canonical reads. Questions, answers,
retrieved bodies, private checkout paths, secrets and tokens never enter URLs.
Resource inspection preserves exact revisions and the return location.

Mount only the current route's feature queries/polling, shared shell metadata
and mandatory checks for currently displayed protected content. Lazy-load feature
bundles. Brain/selection changes cancel superseded requests and clear incompatible
state; late responses cannot replace the new selection. Logout, current access
failure, observed epoch change and advertised expiry clear affected payloads.
Do not retain old protected data next to a failed refresh. Existing bounded
validity polling remains until an equally strong replacement is proven.

### Shared visual language

Use the accepted primitive, semantic and component token layers. The user's
2026-10-03 correction restores the original light Atlas palette and supersedes
the earlier warm-dark direction: warm cream canvas `#f7f4ec`, light paper
surfaces `#fffdf8`, cream sidebar `#eee9de`, dark ink `#101b2a` and sage-green
accent `#3d6b59`. Keep the new motion, pipeline and terminal features, with
readable per-category accents and subtle tints adapted to light surfaces.
Terminal cards and ambient mode use the same light palette. No dark-mode switch.
The exact values live in the design-system document.
Newsreader 500 page headings
are 36–40px/1.1 and section headings 24px/1.2; Manrope controls 14px/1.45 and
reading text 16px/1.55; DM Mono metadata 12px/1.5. Self-host licensed font files
and notices; no runtime external font request. Lucide icons use one shared
size/stroke convention and navigation map rather than per-page variants.

Motion is purposeful and fluid: shared duration/easing tokens (fast ~150ms,
base ~250ms, slow ~450ms, ambient ~1200ms; standard and emphasized easings) with
staggered entrance for lists/cards (~40ms steps), pulsing status while a job is
processing, skeleton loaders during loading, and smooth page-content transitions.
Animations never change what data is shown, never imply precision the feeds do
not have, and honor `prefers-reduced-motion` by degrading to static content. No
decorative 3D rendering. The approved 2026-10-03 control-panel revision permits
Motion for shared layout/transitions and React Flow for bounded read-only
diagrams; CSS/SVG remain appropriate for simple effects. Keep Cytoscape for the
existing knowledge graph. Load diagram dependencies only on their consuming routes.

Agent and tool output renders in a terminal-style card: macOS traffic-light
chrome, monospace body, bounded safe ANSI subset (colors/bold/dim; no cursor
escapes), sanitized input, and the same truncation/redaction as the underlying
detail views. The Rust plugin CLI presents an animated startup banner and live
status lines (spinner, colored outcomes, elapsed time) on a TTY, falling back to
plain output for non-TTY, `NO_COLOR` or CI.

The Brain's Activity surface hosts a live pipeline view — capture → learning →
memory/graph stage nodes with animated edges — that animates state diffs over the
existing polling feeds: new items slide in, active edges pulse, counts tick, and
a processing indicator glows while a learning job runs. It shows only real feed
data: no fabricated counts, streaming text or causal ordering. A fullscreen
ambient mode at `/brains/{brain}/tv` (hidden from navigation, toggled from
Activity) cycles the Brain's recent memories one at a time with slow fade/scale
transitions, category glow, timestamp and provenance; it reads only the existing
authorized memory list.

Create an original vector Recollect mark expressing remembered fragments gathering
together, plus a reusable wordmark/compact icon and SVG favicon. It must remain
recognizable at navigation size, use the same tokens, and have appropriate text
alternatives or decorative semantics. A raster image is not a substitute.

Shared PageHeader, FilterBar, ScopeSummary, DetailInspector, StatusBadge,
AsyncState, SettingsSection and ActionMenu own recurring behavior. Keep forms,
dialogs, tables and graphs on that system. No mobile redesign. Apply the display
and interaction priority above. Keep readable long labels and non-color status
meanings. Auth, invitation, OIDC error/recovery and expired-session views use the
same system.

### Page responsibilities

| Destination | Required behavior and boundary |
| --- | --- |
| Ask | Question/citations/inspector governed by [answers](retrieval-answers.md); the default view is a chat-style conversation thread — user questions and model-grounded answers as turns, with citations as expandable evidence attachments and follow-ups in the same thread — and Search evidence is an explicit alternative mode retaining Matches, Disagreements, Sources, Copy context and advanced exact/mode/time/budget criteria. The thread stays temporary; applied restrictions stay visible. |
| Memory | Readable claim/decision/procedure/handover list and exact detail/history/support. Optional correction, review, withdrawal, conflict resolution and erasure retain canonical handlers. Handover generation selects exact claim revisions, never silently saves Ask text. |
| Sources | Current paste/text-file/reference import, list/version content, collections/areas/environment filters and one Manage views editor. New-version editing and contextual reprocess/learning/erase retain their effects. Show reference-only/unavailable bytes honestly. Storage policy lives in Settings. |
| Graph | Canvas first, compact kind/scope toolbar, filters/insights/status drawers and canonical inspector. Provide keyboard/list selection and textual paths. Default bounded knowledge overview uses current Brain-only investigation selection when no explicit selection exists; it invokes the existing read endpoint once, without model/rebuild/analytics. Explicit historical/repository/manifest selections are never guessed or reset. Missing exact inputs ask for selection. Existing bounds and oversize refusal remain. |
| Repositories | Published repositories, snapshots, files/facts/coverage/contributors/insights/receipt; Environments tab owns exact revision manifests/history. Browser publishing opens accurate native instructions. Own checkout paths remain account/device-private. Committed, desired and observed revisions stay distinct. |
| Agents | Sole home for connecting a coding tool that acts as the signed-in user: supported host onboarding and real memory-read check, published sessions/capture coverage and advanced own task/scope controls. A per-Brain agent roster above the setup tabs shows agents across users that have been observed on this accessible Brain, grouped by user ("mike — N agents"); each row is one connection of exactly two kinds — Recollect plugin or direct MCP access token — with host kind, credential state, last used on this Brain, and an own-credential-only revoke action labeled as removing the keycard from all Brains; revoked/expired rows hide behind an explicit toggle. A link opens the global Agents page for the complete account list with per-Brain usage. Setup is not an active connection. Capture policy has one editor in Settings. Browser metadata retry and native discovery are distinct. Other accounts' tasks remain private even from Brain admins. |
| Connections | Outbound Brain-managed MCP only; no agent-connection setup path. Connections/Profiles/Runners; approved connector → target/environment → credential reference → runner → review → explicit test wizard. Independent Use/Manage/Share grants remain. Test may have effects and never implicitly retries uncertain work. No arbitrary executable installation. |
| Activity | Exception-first drill-down behind the standing assurance band, which renders only while something needs attention — a blocker, pending read, failed feed, archived Brain, disabled autonomous memory, or an empty first session — and stays hidden on a healthy Brain. Timeline/Processing/Tool calls/Model usage compose existing bounded authorized feeds, with canonical detail links. No fabricated counts or total causal ordering. Admin audit, profile outputs and private contexts retain independent authorization. |
| Agents (global) | Roster at `/agents`: every user, including installation owners, sees only their own agents. Group compact rows by accessible Brain with observed use; omit empty Brain groups. Show name, host/integration and last use there; entry point from every per-Brain roster. Pairing approval and complete own-account credentials/history live in the Agents Manage access drawer. `/devices` remains a compatibility redirect, preserving pairing codes. |
| Settings | General/Access/AI & automation/Capture/Retention & privacy, with rare repair/history actions secondary. General owns archive/reopen and irreversible Brain deletion through the preview and confirmation flow in [brain deletion](platform-brain-deletion.md), kept visibly distinct from archive. Installed provider/model identity read-only; permission, content classes, purposes and quotas editable by actual authorized roles. Capture, model transmission, answering and retention remain separate decisions. |
| Brains | Create/open, personal/shared, archived and search views with supported metadata; installation health is a small affordance with owner diagnostics. |
| Team | Existing account/invitation/local/OIDC/disable/reset/audit functions, installation-owner only; Brain access remains a different resource. |
| Devices compatibility | `/devices` redirects into Agents Manage access; `/devices?code=…` opens pairing there, including across sign-in. No separate normal Devices page. Pending approval/deny/revoke, history and own-account boundaries remain. |

Normal autonomous learning, capture and agent scope/tool work require no new
mandatory human clicking. Existing capability controls remain reachable in their
owning page or contextual inspector. Hiding a control does not change its default
or permission. Counts/statuses need actual response support; unavailable data is
not replaced with sample/mock values.

### Live control panel amendment — 2026-10-03

The user approved the flat cream/sage Flow deck and explicitly requires actual
current or last-observed activity, including multiple agents and the information
being processed. This supersedes the earlier presentation-only pipeline limit.
The October 3 browser feedback supersedes the initial global pipeline: Brains is
a chooser with cards that open a Brain directly. Processing stays in per-Brain
Activity. Agents, Team, Runtime diagnostics, creation and
identity dialogs adopt the same flat panels and context-preserving drawers.
Keep all existing real actions, access boundaries and source/model disclosures.

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
or running. Global agent metadata is own-account-only, including for installation owners.
Within an accessible Brain, observed agent identity and last use may span users,
without sharing foreign credential timestamps or revocation authority. Private tasks and unshared prompts remain
outside all these views.

The selected feed refreshes every two seconds while visible. Label the cadence
and last check; do not imply token streaming or stages missed between polls.
Show currently queued/running work distinctly from recent outcomes. Initial
history is static; stable ID/state changes may produce one short path pulse.
No demonstration/replay data or timer-generated activity enters product views.
Failed, expired or forbidden refreshes clear protected payload and halt motion;
selection changes cancel requests and reset the animation baseline. Re-check
effective retention/erasure on every read, omit removed content and never expose
private task context. Pause hidden-page motion and honor reduced motion.

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

### Permissions and failure behavior

Disabled answering leads to exact/text search, not a fake answer. Configured
credentials/models/agents/tools are not reported connected without a successful
corresponding call. Explicit model/tool checks show cost/effect implications.
Never infer profile Use from Brain membership or installation ownership.

Every page handles loading, empty, partial, failed, forbidden, archived and stale
states. Preserve safe unsaved input on recoverable errors, but clear invalid
protected payload. Destructive or effectful actions retain actual previews,
permissions, version checks and uncertainty. This redesign does not reset data,
enable model transmission or change capture/retention policy during migration.

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

## Explicit Deferrals

Mobile/small-tablet redesign, dark mode, arbitrary MCP executable approval,
browser provider installation, broader administrative MCP parity, new source
formats/connectors and general-purpose agent hosting. Saved or action-capable
Ask requires separate authority. No external deployment, automatic version bump,
commit or push is implied by this desktop delivery.

### Brain identity and workspace navigation correction — 2026-10-03

Brain creation/editing supports PNG, SVG and ICO artwork up to 512 KiB. Browser
image-context rendering converts artwork to at most 256px PNG; originals are
never inserted as executable markup or retained. The server independently checks
PNG format, dimensions and decoder allocation, then normalizes the bytes. Store
only bounded PNG bytes and revision on the Brain row, never in receipts or audit.
Reading follows Brain visibility; changing/removing follows Brain admin and
browser CSRF authority. Responses are authenticated, no-store and nosniff.
Whole-Brain deletion and restoration fencing cover the icon through the Brain row.

The global chooser contains no activity preview. Account access/history moves
inside Agents, preserving native pairing URLs and opaque codes. The user's
approved grouped-list Agents design shows only the signed-in account’s agents
globally, including for installation owners. Inside an accessible Brain, safe
agent identity/host/last-use metadata spans contributing users; private tasks,
payloads, foreign credential dates and revocation authority are not shared.
Only real observed use may associate an agent with a Brain. Empty groups are
omitted, while unused credentials remain reachable in Manage access.

### Management simplification — 2026-10-04

The approved [setup and exception management amendment](desktop-knowledge-surface.md#setup-and-exception-management--2026-10-04)
supersedes the earlier Agents session/context tabs and Connections peer runner tab.
Normal Agents is an observed roster with Connect and scoped activity; private
contexts and detailed verification are troubleshooting. Capture receipts are
Activity events with exact evidence, not a synthetic session transcript. Connections
keeps Tool access and explicit execution placement while runner/schema/test details
are secondary. Settings uses General, Access, Privacy and AI permissions, with one
editor per policy and valid custom restrictions. Existing private records, grants,
retention granularity, deletion fences, real-call semantics and safe deep links remain.

## Final desktop polish and ambient acceptance — 2026-10-04

The user authorized final Graph control hierarchy, searchable compact Connections,
an explicit setup link to recorded Brain activity, direct Privacy customization
links and measured initial loading reduction. Existing files retain their roles;
no file-size refactor or new dependency is required. Secondary Graph tools move
behind one menu without hiding applied scope, coverage or failures. Search filters
only authorized loaded connections and distinguishes no matches from no setup.
Setup does not infer success from another agent's activity. Privacy links select
an existing editor without changing policy. Presentation loading may be deferred;
authority refresh, expiry gates and exact inspection remain intact.

Ambient TV reads the existing bounded claim page. That page advertises its earliest
canonical retention deadline as optional `expires_at`, computed from the displayed
claim revisions and supporting evidence whose retained labels were emitted. `knowledge_until` remains
historical version closure, not retention. TV clears every displayed card and
feed timestamp on error, advertised expiry or access loss, and re-reads on re-entry.
It tracks exact revision identities across list insertions, presents review,
freshness, operational and conflict/rule qualifications, pauses while hidden,
honors reduced motion, and provides an explicit exit with isolated focus.
