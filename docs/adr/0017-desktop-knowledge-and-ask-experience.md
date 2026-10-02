# 0017: One knowledge surface, primary Ask and separated wiring

Status: accepted

The user's [2026-10-01 display and interaction priority](../contracts/desktop-experience.md#display-and-interaction-priority--2026-10-01)
supersedes mandatory small-screen and keyboard-polish expectations. Prioritize
normal laptops and larger desktop monitors, up to about 32 inches.

The user's 2026-10-01 assurance clarification limits prominent warnings to
current blockers, with historical failures retained in Activity. The
[assurance contract](../contracts/desktop-knowledge-surface.md#assurance-band)
records the current-status evidence required; an old failed job alone is not a
current blocker.

## Decision

Restructure the Brain experience by visit frequency and human role, not by
which backend epic owns the data. [ADR 0014](0014-desktop-experience-and-answers.md)
remains authoritative for the token system, fonts, SVG identity, route
ownership, desktop-only scope and the read-only answer posture. This decision
changes presentation and placement only.

Four tiers replace nine peer destinations in the sidebar:

| Tier | Contents | Human cadence |
| --- | --- | --- |
| Ask | Question-first answering with exact search as an explicit mode | Daily |
| Knowledge | Memory, Sources, Graph and Repositories as one surface with view switching | Occasional inspection |
| Wiring | Agents, Connections, Team access | Once, then rarely |
| Assurance | Health, exceptions and Activity | Only when something needs attention |

Every existing route stays a real, deep-linkable URL. `/memory`, `/sources`,
`/graph` and `/repositories` continue to resolve and become anchors inside the
Knowledge surface rather than separate destinations. No contracted capability,
permission, inspector or failure state becomes unreachable by moving it.

One shared lineage inspector is the canonical place to understand how knowledge
relates to its evidence. Selecting a source shows what it derived; selecting a
memory shows its supporting evidence and its graph neighbourhood inline. Graph
becomes a view of the current selection as well as a standalone exploration
mode, and its layout, filter, coverage and inspection controls live in the
canvas chrome rather than above it.

Ask leads with the question composer. Exact/text search remains fully available
as an explicit mode and as the fallback when answering is disabled, but it is
no longer the landing tab. This amends the presentation decision recorded in the
shipped [successor cleanup pack](../roadmap/execution/archive/mcp-successor-cleanup.md)
and restores the landing behavior
[desktop contract](../contracts/desktop-experience.md) already specifies.

Incoming and outbound connections are separated without merging their authority.
Agents is the single home for connecting a coding tool that acts as the signed-in
user. Connections drops its Coding agents tab and owns only outbound
Brain-managed MCP servers, tool groups and where they run. This resolves the
contradiction between the desktop contract and the managed-experience amendment
that produced two pages describing one action.

A standing assurance band reports autonomous behavior as an exception surface:
it states what the Brain learned, revised and retired, and escalates only when a
human must act. Activity becomes its drill-down rather than a primary audit
dump. Under an adopted policy the correct ordinary state is "nothing needs you",
and the UI must say so.

Presentation must stop treating all pages as interchangeable. The eyebrow,
large serif heading, subtitle and full-width card stack silhouette is reserved
for Ask. Archive and assurance views use denser lists with a single composite
status mark whose full review, freshness and operational dimensions remain
readable in the inspector and available as text, so the accepted three-state
separation is preserved without fifty-one repeated chips per screen.

## Why

The user decided on 2026-09-29 to merge Memory, Sources and Graph into one
surface and to improve the UI, and to separate Agents from Connections with
differences flagged for later correction.

The evidence is in the delivered product. All twelve destinations render the
same silhouette, so nothing signals relative importance. On Graph the canvas
measured 522 pixels of a 900-pixel viewport while selection, scope, filter,
path, insights, status and a coverage banner stacked above it, inverting the
accepted "canvas is the main desktop area" instruction. Memory spends its entire
hierarchy on three outlined chips per row. Activity presents every audit event
with an identical green tick and a `Details` button, which is why it reads as
volume without value. Sources and Memory never reference each other, so the
question of what a source produced, or what supports a memory, has no answer
anywhere in the interface.

The duplication is a documented conflict rather than an implementation slip.
The desktop contract assigns Connections to outbound connections only, while the
managed-experience amendment places agent connection cards inside it. The live
app therefore offers `Connections → Coding agents` and `Agents → Connect an
agent` as two paths to one operation, while the sidebar promotes Connections at
top level and buries Agents inside a collapsed group, inverted from the order in
which a person actually adopts the product.

Alternatives considered and rejected: keeping nine peer routes and only
restyling, which preserves the equal-prominence problem the user objected to;
deleting the Search mode to simplify Ask, which would remove a contracted
capability and strand readers on a Brain with answering disabled; and merging
outbound and inbound connections into one page, which would confuse device
authority with tool authority precisely where the accepted contracts keep them
independent.

## Consequences

Navigation, tab and inspector behavior needs its own testable contract, so
[desktop knowledge surface](../contracts/desktop-knowledge-surface.md) governs
this decision while the domain contracts retain mutation, grant, retention and
graph-computation authority. The Knowledge surface is a coordinated presentation
layer: memory, evidence, graph and retrieval owners keep their data, handlers
and proof obligations, following the coordination precedent the desktop
contracts slice already established for the platform epic.

Six in-progress desktop packs gain scope. Their recorded acceptance criteria
stay valid and their packs must be reconciled rather than rewritten, so the
surface work is specified as successor slices that name those packs as
predecessors.

Accessibility obligations rise. Collapsing four pages into one surface and
compressing status into a mark must not lose non-color status meaning, keyboard
reachability, focus return or the textual graph alternative the desktop contract
requires. Every hidden control keeps its existing default and permission; hiding
is never a policy change.

The assurance band introduces no new aggregation authority. It composes only
already-authorized bounded feeds, and any count it displays must map to a
verified response field or an explicitly scoped read endpoint rather than a
client-side estimate.

## Supersession

Amends [ADR 0014](0014-desktop-experience-and-answers.md) by narrowing its
nine-peer navigation enumeration to the tiered layout above while leaving its
token, typography, icon, route-ownership, desktop-only and read-only-answer
decisions intact. Supersedes the Ask landing-tab decision recorded in the
shipped [successor cleanup pack](../roadmap/execution/archive/mcp-successor-cleanup.md)
and the agent-connection-card placement in
[managed experience](../contracts/memory-managed-experience.md#human-and-agent-experience).
No domain contract, permission, retention rule or retrieval semantics is
superseded.
