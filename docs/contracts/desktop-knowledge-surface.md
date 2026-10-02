# Tiered desktop knowledge, Ask, wiring and assurance experience

Status: accepted

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

The Brain sidebar presents four tiers in this order: Ask, Knowledge, Wiring and
Assurance. Tier labels are visible group headings. Team remains
installation-owner only and stays a global destination alongside Brains and
Devices.

Every contracted route keeps its existing path and remains deep-linkable,
reloadable and reachable through Back and forward. `/memory`, `/sources`,
`/graph` and `/repositories` render inside the Knowledge surface with the
requested view selected and validated URL state preserved. An unknown view value
falls back to the documented default. `/brains/{brain}` resolves to Ask.
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
navigation. The keyboard-accessible list alternative remains required and is
reachable from the canvas chrome. Layout choice is presentation only and never
implies a relationship the data does not contain.

### Ask primacy

Ask leads with the question composer as the default view. Exact/text search is
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

Captured sessions and working contexts remain under Agents. Task scope records
stay account-private and invisible to other accounts including Brain admins.
Capture configuration keeps its single canonical editor in Settings; Agents
reports effective coverage and links there.

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
- Ask opens on the question composer by default and by `/brains/{brain}`;
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
