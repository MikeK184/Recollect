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
- [Design system document](../../foundation/design-system.md)
- [claude-mem study, 2026-10-03](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/research/claude-mem-study-2026-10-03.md) (reference for terminal-card, TV and live-feed visual language; not implementation authority)
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
| `desktop-grouped-3d-graph` | shipped | adr-backed, contract-backed | archived | User-authorized grouped 3D navigation, shared qualified 2D data, stable finite layout, accessibility/fallback, bounded exact-generation status tracking and performance proof |
| `desktop-dashboard-refresh-stability` | shipped | contract-backed | archived | Preserve fresh pipeline layout across visibility and metadata refreshes without extending validity |
| `desktop-input-errors-and-page-recovery` | shipped | contract-backed | archived | Clear learning failure reasons, guarded input diagnostics and stale page recovery |
| `desktop-inline-processing-limits` | shipped | contract-backed | inline: explicit placement correction to existing controls and scoped CSS; canonical fields, bounds and Save/Cancel unchanged | Move Processing limits into the main AI policy form with a responsive two-column grid |
| `desktop-private-runner-relationships` | shipped | contract-backed | archived | Approved device/connection/tool/group cards with reader-safe binding summaries and independent UI acceptance |
| `desktop-private-runners-spacing` | shipped | contract-backed | inline: explicit alignment correction to existing scoped CSS; registration, device binding and authority unchanged | Align Private Runners with Tool access and reduce excess tab-to-toolbar spacing; relationship display remains a proposal |
| `desktop-inline-private-runners` | shipped | contract-backed | archived | Approved compact cards and inline Add/Edit delivered locally with immutable device, authority and revision guards |
| `desktop-private-runners-label-concepts` | shipped | contract-backed | inline: explicit tab/copy rename and concept-only images; routes, registration and editor behavior unchanged | Private Runners label plus compact read and inline Add/Edit proposals |
| `desktop-tool-access-header-badges` | shipped | contract-backed | inline: explicit badge/alignment correction in one existing header; canonical draft/save behavior unchanged | Separate environment/MCP badges and keep environment editing beside its metadata |
| `desktop-compact-tool-access` | shipped | contract-backed | archived | Approved tighter cards, single permission rows and source popovers; canonical retry and independent laptop/wide acceptance |
| `desktop-readable-mcp-inspection` | shipped | contract-backed | archived | Scope/runner badges, reliable tool-read states, single formatted descriptions/JSON and structured call inspection |
| `desktop-general-inline-environments` | shipped | contract-backed | archived | Selected General layout and stable inline environment add/rename over existing commands |
| `desktop-bounded-privacy-rows` | shipped | contract-backed | inline: explicit width/separator correction and removal of two presentation controls; existing draft payloads and commands unchanged | Cap Privacy width, highlight focused rows and hide manual exclusions while preserving saved rules |
| `desktop-unified-privacy-form` | shipped | contract-backed | archived | Single Privacy form with title separators and shared edit actions, preserving separate canonical saves and truthful recovery fields |
| `desktop-visible-management-settings` | shipped | contract-backed | archived | Seven follow-up corrections: bottom Team, concise inspection and visible AI/Privacy controls with optional recovery diagnostics |
| `desktop-consistent-management-scale` | shipped | contract-backed | archived | Shared shell scale, persistent diagnostics and removal of automatic-context navigation |
| `desktop-readable-management-content` | shipped | contract-backed | archived | Compact agent activity and safe highlighted management code/Markdown |
| `desktop-inline-management-editors` | shipped | contract-backed | archived | Stable inline connection/AI editing and Connections-style tool-group cards with adjacent permission icons |
| `desktop-management-vision-fidelity` | shipped | contract-backed | archived | Correct six management screens to the approved vision, with independent rendered comparison |
| `desktop-management-concepts` | shipped | contract-backed | archived | Implement six user-approved management concepts, global connector library and bounded credential/config setup, with independent before/vision/runtime UI review |
| `motion-design-foundation` | shipped | contract-backed | archived | Warm dark token system with per-category accents, motion tokens and shared animation primitives (entrance stagger, pulse, skeleton, page transition) applied to the core shell |
| `live-pipeline-visualization` | shipped | contract-backed | archived | Animated capture → learning → memory/graph pipeline view on the Activity surface with processing/queue indicators, animating state diffs over existing polling feeds |
| `terminal-agent-surface` | shipped | contract-backed | archived | Terminal-card component for agent/tool output in the web UI plus an animated banner and status presentation in the Rust plugin CLI |
| `desktop-light-palette-restoration` | shipped | contract-backed | inline: explicit restoration of the existing palette and its presentation adapters; no new behavior or interfaces | Restore cream/ink/sage, preserve motion/pipeline/terminal features, adapt category and ANSI foregrounds for light surfaces |
| `memory-tv-ambient` | shipped | contract-backed | archived | Fullscreen ambient mode cycling recent memories with slow fade/scale transitions, toggled from Activity |
| `desktop-final-polish` | shipped | contract-backed | archived | Compact Graph/Connections controls, explicit setup activity destination, direct Privacy editing and measured lazy presentation loading |
| `desktop-actionable-management` | shipped | contract-backed | archived | Generic connection setup and explicit tests, concise tool access, member list/popups, inline privacy, optional environments and automatic learning presentation |
| `desktop-graph-led-workspace` | shipped | contract-backed | archived | Main 3D/2D Graph, bounded contribution perspective, Explore and simplified rich Ask |
| `desktop-flat-graph-correction` | shipped | contract-backed | inline: explicit rejection of 3D with an existing accepted 2D replacement; no new interface or behavior | Remove rejected 3D presentation and reuse the existing 2D renderer; assess Graphify viewer |
| `desktop-graph-interactions` | shipped | contract-backed | archived | Smart labels, truthful local type filters, focused connections and compact evidence summary |
| `desktop-guided-setup-and-controls` | shipped | contract-backed | archived | One-step agent setup, compact contexts, connector choice and automatic privacy |
| `desktop-management-simplification` | shipped | contract-backed | archived | Contributor-first Agents, contextual activity and diagnostics, tool access and clear privacy/AI permissions |
| `desktop-brain-pages` | shipped | contract-backed | archived | Approved Dashboard landing, concise Ask and scalable Knowledge inspection |
| `desktop-brain-identity-and-navigation` | shipped | contract-backed | archived | Focused Brain chooser, uploaded icons, personal grouped Agents and Brain-scoped contributors; access drawer and pairing redirects |
| `desktop-live-control-panel` | shipped | contract-backed | archived | Approved flat light global pages and real source/agent/job pipeline with exact attribution, freshness and failure handling |

The archived motion foundation records its original dark-theme delivery; the
light-palette restoration supersedes that color choice without reopening its
motion acceptance. Ambient TV and final polish are accepted and archived as of
October 4; the closeouts below supersede earlier remaining-work notes. The
subsequent actionable management correction is also accepted and archived.

## Current status

The light desktop, Ask/chat, pipeline, memory TV and current management layouts are delivered. Grouped 3D, exact generation tracking, shared 2D fallback and source-linked topic/entity navigation are locally delivered and independently validated. Production automatic organization quality remains with graph intelligence; the normal installation has not been updated. Optional small-screen polish, SSE and other product experiments are not missing acceptance for these completed desktop slices.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Archived packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) retain the delivered slices.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/desktop-visual-experience.md).
Dated test/deployment observations are not a current runtime health claim.
