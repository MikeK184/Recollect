# Tiered desktop knowledge, Ask, wiring and assurance experience

Status: accepted

### Approved management concepts — 2026-10-05

The approved October 5 concepts supersede the October 4 management presentation:
Connections/Tool access/Runners are peer views within Connections, with private
execution still separate from agent memory setup. Settings AI permissions always
loads the bounded current semantic coverage summary; diagnostics, policy history
and batch history load only when explicitly opened. Global navigation remains
visible within a Brain and adds the installation-owner connector library.
Privacy editing preserves the same rows and all distinct canonical policy fields.
Other authority, polling/revocation and private-context boundaries stay in force.

The [October 3 graph-led workspace amendment](#graph-led-daily-workspace--2026-10-03)
and [October 4 management amendment](#setup-and-exception-management--2026-10-04)
define current navigation and presentation. Earlier tier descriptions
retain their history; domain authority and exact inspection remain unchanged.

The user's [2026-10-01 display and interaction priority](desktop-experience.md#display-and-interaction-priority--2026-10-01)
applies throughout this contract: laptop and larger desktop use is primary;
small-screen and additional keyboard/focus work are optional, not delivery gates.

## Source

[ADR 0017](../adr/0017-desktop-knowledge-and-ask-experience.md) records the
decision. The user approved the merged knowledge surface, the improved visual
presentation and the Agents/Connections split on 2026-09-29.
[Desktop experience](desktop-experience.md) retains authority over tokens,
typography, icons, the SVG identity, route ownership, desktop widths and
accessibility. [Temporary answers](retrieval-answers.md) governs question
behavior and requires exact/text search to remain available.
[Memory claims and time](memory-claims-and-time.md),
[review and corrections](memory-review-and-corrections.md),
[evidence collections](evidence-collections.md),
[graph exploration](graph-exploration.md) and
[MCP catalogue and profiles](mcp-catalogue-and-profiles.md) retain their domain
authority. [Vision: one memory model](../foundation/vision.md#one-memory-model-complementary-methods)
defines representations as projections rather than independent truths.

## Contract

### Tiers and routes

The Brain sidebar presents four tiers in this order: Brain (Dashboard and Ask), Knowledge, Wiring and
Assurance. Tier labels are visible group headings. Team remains
installation-owner only and stays a global destination alongside Brains and
Devices.

Every contracted route keeps its existing path and remains deep-linkable,
reloadable and reachable through Back and forward. `/memory`, `/sources`,
`/graph` and `/repositories` render inside the Knowledge surface with the
requested view selected and validated URL state preserved. An unknown view value
falls back to the documented default. `/brains/{brain}` resolves to Dashboard. Explicit Ask and Activity routes remain.
Nothing in this contract changes which handler authorizes or mutates data.

The Knowledge surface owns one view switcher and one shared inspector region.
Switching views preserves the current selection when the selection is meaningful
in both views and clears it when it is not, never silently substituting an
identifier across views or Brains.

### Lineage inspector

One canonical inspector explains how evidence, memory and structure relate.

- Selecting a memory shows its readable content, its supporting evidence with
  exact versions and locations, and its bounded graph neighbourhood inline
  beneath the evidence.
- Selecting a source shows its versions, availability and what it currently
  supports, with a link into each derived record.
- Selecting a graph node or edge reuses the same inspector and does not fork a
  second copy of identity, evidence or applicability.
- Derived records are labelled as derivations. A graph edge, summary or embedding
  is never presented as independent support, and retrieving a procedure never
  grants execution.

The inspector preserves the separate review, freshness and operational dimensions,
fact time versus knowledge time, provenance, correction, withdrawal, rejection
and contextual erasure through the canonical handlers. Missing source,
unavailable bytes, disputed memory, stale revision, erased content, archived
Brain and lost access keep their explicit states. A compact view may summarize a
dimension but must never merge or reorder them.

### Graph placement

Graph controls live in the canvas chrome: kind selection, entity search, scope
chips, layout, zoom, fit, focus, filter, insights and status. The canvas occupies
the dominant region of the Knowledge surface at the tested desktop widths rather
than sitting below stacked forms. Coverage and eligibility limitations collapse
to a single persistent indicator that expands on demand, and remain visible when
collapsed.

The bounded default read stays exactly as
[desktop experience](desktop-experience.md) specifies: one existing read
endpoint, no model call, no rebuild, no analytics and no arbitrary traversal on
navigation. An optional Entities finder inside canvas chrome provides textual selection of
displayed nodes and separately paginated eligible entities; it never replaces
the canvas with a competing List view. Layout choice is presentation only and never
implies a relationship the data does not contain.

### Ask primacy

Dashboard is the default Brain view. Ask leads with the question composer when
selected. Temporary independent questions are labelled explicitly. Exact/text search is
an explicit mode, reachable directly by URL and always available without a model
call. When answering is disabled or unconfigured, Ask presents search and explains
what an administrator can configure, rather than failing silently.

Applied scope, time and non-default criteria remain visible above results in both
modes. Question text, answers, retrieved bodies, private paths and credentials
never enter URLs. Conversation state stays temporary per
[temporary answers](retrieval-answers.md); this contract adds no persistence.

### Wiring split

Agents is the only destination for connecting a coding tool that acts as the
signed-in user, and covers host selection, pairing when required, Brain binding,
workspace configuration, host setup and a real memory-read check. Connections
owns outbound Brain-managed MCP servers, tool groups and where they run, and
offers no agent-connection card or duplicate setup path. Existing
`/connections?tab=coding-agents` style state resolves to the Agents destination
rather than rendering a second copy.

The October 4 management amendment below supersedes the original Agents tabs.
Task scope records stay account-private and invisible to other accounts including
Brain admins. Capture configuration keeps its single canonical editor in Settings;
Agents reports observed use and links to authorized activity.

Where-it-runs placement is explained in plain language inside Connections: an
approved connection executes on the central service, on a paired device, or on an
approved private-network runner, and the choice determines which network can reach
the target. Device identity remains global. Terminology in the running product
names this concept consistently and links to the paired device record rather than
presenting an undefined noun.

### Assurance band

A single standing band summarizes autonomous behavior for the current Brain:
what was learned, revised, retired and captured, plus whether anything needs a
human. The user's 2026-10-01 clarification limits prominent warnings to current
blockers: an enabled automation policy missing its provider credential, canonical
processing status reporting failed with no queued work, an unresolved uncertain
tool outcome, or erasure reporting an error. Historical failed jobs/tool calls,
pending cleanup and reported capture gaps remain available in Activity; those
records alone do not establish that automatic work is currently blocked. Do not
infer recovery or current failure from an arbitrary age cutoff. Name the current
problem in the banner and link to its diagnostic surface; omit unrelated learning
and capture counts while warning. The ordinary state under an adopted policy states that no
action is required.

Every displayed figure maps to a verified response field or an explicitly scoped
authorized read endpoint. The band composes existing bounded feeds, adds no
aggregation that widens visibility, never claims a total causal order across
independent subsystems, and does not leak account-private scopes or paths.
Activity becomes the drill-down from the band and keeps its exception-first
ordering, with the full authorized audit feed reachable but not primary. Readers
see only what their role permits; admin audit stays admin-only.

A configured credential, saved connection, installed model or rendered setup card
is never reported as connected or healthy without a successful corresponding call.

### Presentation density

The eyebrow, oversized serif heading, subtitle and full-width card stack pattern
is reserved for Ask. Knowledge and Assurance views lead with content and use
denser lists. Memory, Sources and Repositories lists present one primary
identifier line plus at most one status line, with the full dimension set in the
inspector. Status uses one composite mark whose meaning is also available as
text, so no distinction depends on color alone.

Repository origins render as canonical normalized form with overflow handled
without truncating identity meaning. Machine identifiers and UUIDs move to the
inspector or a copyable field and stop occupying list rows. Device and Brain
lists order active records first and collapse revoked or historical records
behind an explicit filter. Empty, filtered-empty and no-results states are
distinct.

Recurring headers, async states, status marks, filters and the inspector region
come from the shared component set named in [desktop experience](desktop-experience.md).
No page may fork a private copy of the lineage inspector.

### Boundaries

Hiding, relocating or summarizing a control changes presentation only. Every
existing default, permission, policy, retention rule, budget, preview, version
check and uncertainty handling survives unchanged, and any control that moves
remains reachable in its new location. No view change enables model transmission,
starts a job, calls a provider, executes a tool or writes memory.

Keyboard navigation, focus return, accessible names, readable long labels,
reduced motion, measured contrast and the tested 1280, 1440 and 1920 widths apply
to every new arrangement. Brain switching cancels superseded requests and clears
incompatible selection and inspector state; late responses cannot populate the
new Brain.

## Acceptance

- All four tiers render with real route navigation, and every pre-existing
  contracted URL deep-links, reloads and survives Back and forward into the
  correct view and selection. Unknown view values fall back to the documented
  default.
- Selecting a memory reveals its supporting evidence and bounded neighbourhood;
  selecting a source reveals what it supports. Both use one inspector
  implementation, verified by source inspection for a single definition.
- Graph canvas region exceeds the combined height of its chrome at 1440 by 900,
  with coverage limitation visible while collapsed, and the bounded default
  issues exactly one existing read with no model call, rebuild, analytics or
  traversal, asserted by request capture.
- Dashboard opens by `/brains/{brain}`; Ask opens on the question composer;
  search remains reachable by explicit mode and direct URL, and works with
  answering disabled. No navigation or tab switch triggers a provider request.
- Connections offers no agent-connection setup path; Agents offers one complete
  flow ending in a real memory-read check. Legacy connection tab state resolves
  to Agents.
- The assurance band matches authorized feed counts, escalates only on an
  injected authorized exception, states no-action-required in the ordinary case,
  and hides figures a reader role may not see. A configured-only connection is
  not shown as connected.
- Density changes keep all three status dimensions separately readable as text,
  preserve keyboard reachability and focus return, and pass the tested desktop
  widths with long-label controls.
- Existing domain, retrieval, graph, MCP, retention and access regression suites
  pass from their new locations. Web typecheck, affected browser specs,
  `./scripts/validate.sh` and `git diff --check` run, and request volume is
  measured against the current comparable fixture rather than claimed lower by
  layout.

## Explicit Deferrals

No new persistence, saved conversations, action-capable Ask, background refresh,
cross-Brain aggregation, unified feed endpoint, new role, new authorization layer
or framework change is introduced. Dark mode, mobile and small-tablet layouts stay
deferred by the desktop contract. Renaming product terminology beyond the
where-it-runs and tool-group explanations requires its own decision. Deleting the
Search mode, the mandatory review inbox, adaptive decay and any automatic
executable skill learning remain excluded.

## Approved Brain pages amendment — 2026-10-03

The user approved the reviewed light-theme proposal. Dashboard composes the
existing authorized pipeline; only observed processing and state changes animate.
Completed history stays still. Recorded dispositions may overlap and are not
current inventory. Memory rows show assertion values, contributor and date with
exceptions visible. Sources use one inspector for content, derived records,
versions and canonical actions. Exact older versions are not replaced with current
versions. Repositories use searchable server pages of 50 and exact identity reads;
private checkouts remain secondary and account-private. Snapshot facts show their
actual structured content with raw JSON secondary. Existing scopes, time dimensions,
correction, deletion, failed-read clearing and retention deadlines remain.

Browsing preserves row identities while inspecting; payload always comes from the
latest authorized response. New rows wait behind an explicit indicator; removed
rows and failed reads clear immediately. Free-text searches stay out of URLs. No
new hashes, persistence, rendering library, provider request or role is introduced.

## Graph-led daily workspace — 2026-10-03

Primary destinations are Dashboard, Ask, Graph and Explore. Secondary Manage
contains Agents, Connections and Settings; Dashboard exposes Activity history.
Legacy memory/source/repository routes remain resolvable and select Explore in
the sidebar. Explore mounts one bounded browser at a time, preserving canonical
record identities, exact versions, correction/erase permissions and shared
inspection. Its Memory/Sources/Repositories switch is browsing scope, not three
new main destinations. Memory types are optional filters; handover tools remain
secondary. Graph remains a distinct main route.

Ask uses explicit Answer/Find evidence intent, visible query and compact scope;
advanced retrieval controls are collapsed. Answers and exact retrieved evidence
render safe Markdown/GFM with code copying, bounded lazy highlighting and table
scrolling. Raw HTML and remote image execution are not enabled. Citation IDs
remain server-owned interactive controls. Repeated coverage qualifications are
combined into Evidence notes, while material disagreement/incomplete support
remains visible. Failed requests, lost access and invalidation remain blocking.

## Setup and exception management — 2026-10-04

The user approved the reviewed management simplification. Normal navigation stays
Dashboard, Ask, Graph and Explore; Manage contains Agents, Connections and Settings.
Agents defaults to the observed contributor roster and a single Connect agent flow.
Selecting a contributor opens authorized activity; global Agents remains own-only.
Own credential revocation states its all-Brains effect. Credential validity,
recorded use, capture delivery and successful calls remain separate observations.

Working contexts leave ordinary navigation and onboarding. Preserve native task,
scope and binding APIs, and account-private inspection in Troubleshoot. Advanced
Change selection and Close task are corrections; browser task creation, operation
binding and the misleading Start subagent action leave this surface. Verification
is based on observed calls, with manual read checks secondary. A selected Brain URL
is not a Brain-restricted credential.

Captured events appear in Activity and scoped agent details, with bounded server
pagination. Do not invent a transcript or session grouping. Exact source versions
open through existing authorized evidence reads. Technical receipts, coverage,
host versions, scope and deadlines are secondary. Contributor visibility never
exposes foreign private tasks/checkouts or restricted tool results. Missing history
means no observed record in the displayed scope, never proof of no prior use.
Activity uses one compact filter for its historical/diagnostic views, rather than a
subsystem tab wall; Dashboard remains the primary pipeline.

Connections defaults to saved tools, Add connection and Tool access. Named groups,
connection membership and independent Use/Manage/Share rights remain in Tool access.
Execution placement belongs to connection configuration; shared runner management
is secondary, not a peer tab. Cached schemas, explicit effectful tests, uncertain
outcomes and runtime leases belong in Troubleshoot. No fallback placement or
automatic replay/test is introduced. Configured is not connected; only authorized
observed calls may establish a successful call and its timestamp.

Settings has General, Access, Privacy and AI permissions. Privacy combines capture
consent/exclusions, document and selected repository text storage, and retention.
One canonical editor per policy preserves its current change ID and all distinct
retention/content/purpose/limit fields. Disabling future storage never erases prior
evidence. Mandatory redaction cannot be disabled. Resource tuning, index repair,
policy history and embedding batches are secondary. Restricted policies are valid
Custom permissions; Off and Configured are permission states, not runtime health.
Broad defaults remain optional explicit permission changes, never automatic repair.

Legacy session/context/runner/capture-settings URLs preserve the Brain and opaque
identities into their corresponding activity, private troubleshooting or Privacy
view. Unavailable or denied records are not replaced by another record. Lazy closed
panels perform no diagnostic reads; current access/policy refresh and revocation
handling remain active where data is displayed. Use existing light theme, motion
and components, bounded lists and short-desktop outer scrolling. No new runtime,
animation dependency, schema or authorization model is selected.

## Guided setup and automatic privacy — 2026-10-04

Agent setup shows one stage at a time in one dialog. Plugin is the recommended
method for automatic capture/recall; direct MCP is the explicit tool-only path.
Navigation and copying instructions do not create credentials or prove connection.
Direct credential creation remains explicit and retains its one-time result across
Back/Next until closure. Optional execution runners stay separate from memory setup.

Private context inspection uses compact Open/Closed rows and selected details.
Checkouts are account-private device observations; repository identity and published
evidence are shared according to Brain access. Context selection never grants access.

Connections uses one Add entry: reuse an approved connector, or register a new
definition with installation-owner authority. Configuration, observed SDK sessions
and recorded successful calls remain separate. Bounded runtime reads cannot prove
complete absence. A ready SDK session does not prove a remote process is running;
remote service shutdown is unsupported. Pause/enable changes future connection use
under the existing revision fence; dispatched work may finish.

Privacy normally displays the actual capture, storage and per-class retention
policy with immutable secret redaction. Recommended defaults require no setup.
Custom/off policies remain unchanged; a secondary customization surface retains
the canonical separate policy saves. AI transmission consent remains independently
visible and controlled. No combined save implies atomic writes across these APIs.
