# Desktop Visual Experience: Animated Modern Desktop UI

Status: active

## Purpose

Make the desktop product feel alive and modern in the visual language the user
clarified on 2026-10-03: the original light cream/ink/sage theme with per-category
accents, fluid purposeful motion, terminal-style agent output surfaces, an
animated live pipeline view of capture → learning → memory/graph flow, and an
ambient fullscreen memory display. Every existing function, route, data feed and
permission is preserved. The approved control-panel revision adds a bounded
read projection for truthful activity presentation. Desktop-first; no mobile redesign.

On 2026-10-19 the user approved a 2026 redesign
([ADR 0025](../../adr/0025-two-theme-desktop-redesign.md)): Light and Ink themes
on OKLCH tokens, Tailwind CSS v4 and shadcn/ui in place of Mantine, Mona Sans,
JetBrains Mono and Newsreader, an inset shell with a docked inspector and a ⌘K
command palette. The cream/ink/sage interface described in the shipped slices
below remains the delivered state until the redesign slices ship.

## Governing Sources

- [Desktop contract](../../contracts/desktop-experience.md) (amended 2026-10-03: visual language and motion; 2026-10-19: two-theme redesign)
- [Design system document](../../foundation/design-system.md)
- [claude-mem study, 2026-10-03](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/research/claude-mem-study-2026-10-03.md) (reference for terminal-card, TV and live-feed visual language; not implementation authority)
- [ADR 0014: desktop experience and answers](../../adr/0014-desktop-experience-and-answers.md)
- [ADR 0025: two-theme desktop redesign](../../adr/0025-two-theme-desktop-redesign.md) (amends ADR 0014 and ADR 0017)
- [Knowledge surface contract](../../contracts/desktop-knowledge-surface.md) (amended 2026-10-19: inspector, Ask hero, themes)

## Dependencies and Boundaries

The motion foundation slice lands first; the pipeline view, terminal surfaces and
ambient mode build on its tokens and primitives. The user-approved control-panel
revision adds one bounded server read projection for exact activity correlations
and permits Motion/React Flow for the approved flat design. Earlier shipped
slices retain their original narrower scope. SSE broadcast remains deferred;
the selected feed explicitly refreshes every two seconds. All surfaces keep
their data, routes, URL state, permissions and failure
behavior; presentation changes never change what a page measures or authorizes.

The two-theme redesign is presentation only and needs no execution pack: it
changes no schema, adds no external integration or paid evaluation, and changes
no security or authorization behavior, and its decisions are closed. Choosing
Radix or Base UI as the one primitive family is an implementation choice made
once at the start of the work. `desktop-tailwind-design-check` lands first, so
the design check guards the migration from its first commit; the migration then
replaces tokens and theme, the shell, and each page in bounded steps, keeping
every existing Playwright and domain regression meaningful. The public site in
`site/` is outside this work until the user decides otherwise.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `desktop-tailwind-design-check` | shipped | adr-backed, contract-backed | inline: `web/scripts/check-design.mjs` rejects arbitrary colour values in class names, colour utilities naming none of the 83 pinned token roles (default palette, `white`, `black`), inline `style` colours and raw colour literals including `oklch(…)` outside `web/src/design/`, and checks the Mona Sans, JetBrains Mono and Newsreader manifest entries and OFL notices; validation: seeded violations fail, the clean tree passes `check:design`, web `typecheck` and `./scripts/validate.sh` | The design check now guards the Tailwind migration: `node --test web/scripts/check-design.test.mjs` passes 20/20 (a clean fixture and one seeded violation of each kind); Mona Sans and JetBrains Mono are pinned from google/fonts `23e54b5` with sha256 entries and OFL notices but are not used yet |
| `desktop-two-theme-redesign` | in-progress | adr-backed, contract-backed | inline: every desktop page migrated to the version 2 design system per ADR 0025 and the desktop contract's 2026-10-19 acceptance (Light and Ink with system default, remembered choice and no flash; OKLCH shadcn tokens with jade reserved for AI; Mona Sans, JetBrains Mono and Newsreader for answers, Ask hero and wordmark only; 1440px floor, inset panel, 400px docked inspector or sheet below 1440px, ⌘K palette with Ask hand-off; serif hero on Ask only) with no Mantine dependency and no change to routes, URL state, API calls, polling, permissions or failure states; validation: web typecheck and build with the updated design check, Playwright suites, real-browser review of every destination in both themes at 1440×900, 1536×864, 1728×1117 and 2560×1440, reduced-motion and offline-font checks, and `./scripts/validate.sh` | Delivered locally and uncommitted on 2026-10-20: Mantine and the hand-written feature CSS are gone (no `@mantine` dependency), a shadcn/ui kit on Radix (`web/src/components/ui`) themes Light and Ink from `web/src/design/tokens.css` with the theme applied before paint, the inset shell has the 240/56px sidebar, ⌘K palette with Ask hand-off and the docked 400px inspector (sheet below 1440px), and every destination is migrated. Evidence: real-browser audit of 16 destinations at 1440×900, 1536×864, 1728×1117 and 2560×1440 in both themes (128 captures, 0 layout problems, 0 errors, 0 Mantine nodes; 32 reduced-motion captures clean); pre-paint theme 4/4; kit interaction checks 19/19; web typecheck, build, `check:design` and `./scripts/validate.sh` pass; Playwright compared with a HEAD baseline passes 100/154 tests against 98/154, 4 newly passing. Remaining: `graph-generation` "fresh camera" is intermittent after the redesign (4/4 at HEAD, about half after) because the 3D canvas can install its custom forces after the first simulation tick; 52 tests fail both at HEAD and after (stale specs) and need reconciling |
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
| `desktop-session-revalidation` | shipped | contract-backed | inline: a 401 triggers one shared /api/auth/me check and ends the session only if /me also rejects it, restoring the documented expired-session notice; Playwright for a forced 401, a server-revoked session and an explicit sign-out | A stale or single-route 401 no longer signs out a valid session; a revoked session shows the sign-in view with a "Your session ended" notice and an explicit sign-out shows none; `session.spec.ts` passes 4/4 and 2 of its 3 browser cases fail on the previous code |

The archived motion foundation records its original dark-theme delivery; the
light-palette restoration supersedes that color choice without reopening its
motion acceptance. Ambient TV and final polish are accepted and archived as of
October 4; the closeouts below supersede earlier remaining-work notes. The
subsequent actionable management correction is also accepted and archived.

## Current status

The Light and Ink redesign ([ADR 0025](../../adr/0025-two-theme-desktop-redesign.md))
is delivered locally and uncommitted: `desktop-tailwind-design-check` shipped, and
`desktop-two-theme-redesign` is in progress until the intermittent 3D fresh-camera
regression is fixed (see its row); the normal installation still runs the
previous build. The light desktop, Ask/chat, pipeline, memory TV and current management layouts are delivered. Grouped 3D, exact generation tracking, shared 2D fallback and source-linked topic/entity navigation are locally delivered and independently validated. Production automatic organization quality remains with graph intelligence; the normal installation has not been updated. Optional small-screen polish, SSE and other product experiments are not missing acceptance for these completed desktop slices.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/desktop-visual-experience.md).
Dated test/deployment observations are not a current runtime health claim.
