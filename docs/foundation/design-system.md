# Accepted desktop design system

Status: accepted
Evidence date: 2026-10-19 (version 2 redesign); originally 2026-09-26
Accepted: 2026-09-26, by explicit user approval and implementation request.
Amended: 2026-10-03, restoring the light cream/ink/sage palette; 2026-10-19,
replaced by version 2 (Light and Ink themes, Tailwind CSS v4 and shadcn/ui) on
the user's approval, decided in [ADR 0025](../adr/0025-two-theme-desktop-redesign.md);
2026-10-20, extended to the public site and README.
Scope: desktop browser UI in Light and Ink, 1440px and wider; implementation
follows the desktop contract. Since 2026-10-20 the public site and README also
use these tokens (see [Public site and README](#public-site-and-readme)).

The [desktop contract](../contracts/desktop-experience.md) makes these tokens
and shared visual rules authoritative. This document records version 2. The
version 1 cream/ink/sage system that the shipped interface still uses is
retained at [commit a0aea9d](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/foundation/design-system.md)
until the redesign slice ships.

## Reference and authority

The user approved version 2 of the
[Recollect design system](https://claude.ai/artifact/MpZJ2m2L2D5GkNhgQfeFV6)
on 2026-10-19 (published version `1792413783-6f3b`; its `project/tokens.json`
had sha256 `a9bc04e7ff33482839711ab9e5a8a15a227b2be52503ad4b648123a20b8e9d7b`).
Its README, motion, implementation and 31 component specifications are the
source of this record. When the implementation lands, the generated
`web/src/design/tokens.css` becomes the repository's exact token source; until
then the values below and that pinned artefact govern.

Keep the accepted [stack](techstack.md) as amended: React 19, TypeScript, Vite,
TanStack Router and Query, openapi-fetch, Playwright and Rust static
same-origin delivery stay; Tailwind CSS v4 and shadcn/ui replace Mantine and the
hand-written feature CSS. Atlas, the source of the version 1 palette, remains a
research reference and no longer supplies colours.

Generated concepts and previews do not enforce measured pixels or exact font
rendering. Written tokens and implementation acceptance govern; invented dates,
icons, counts and labels in a preview are not API or schema decisions.

## Voice

Plain, specific and calm. Say what happened, what still works and what to do
next; never apologise or blame, and use no exclamation marks or emoji. Write to
"you"; Recollect never says "I", and AI output is labelled as an answer from a
Brain, not a persona. Sentence case everywhere; product nouns (Brain, Ask,
Explore, Graph, Memory, Sources, Agents, Connections, Activity, Private Runners)
are capitalised. Actions are verb-first and name the result; pending labels end
with an ellipsis. Never show 0 or a dash for data that failed or is pending.

## Tokens and themes

Three layers remain: primitives, semantic roles and component roles. Semantic
colours use shadcn's variable names so one stylesheet themes shadcn components
and Tailwind utilities together; Recollect adds `brand`, status, `kind-*`,
terminal and ANSI roles. Feature code uses roles, never primitives.

`tokens.css` is unlayered and imported before Tailwind. The Tailwind theme maps
every role with `@theme inline` and clears the default palette with
`--color-*: initial`, so utilities such as `bg-red-500` do not compile. Radius,
shadow, easing and font variables override Tailwind's scales by name
(`rounded-lg` is 12px, `font-mono` is JetBrains Mono). Themes switch with
`data-theme="light"` or `"dark"` on `<html>`, defaulting to the system setting,
remembering an explicit choice and applying it before first paint.

Surfaces step up: `background` is the canvas the sidebar sits on; `card` is the
inset panel, cards, inputs and tables; `popover` is menus, dialogs, the palette
and toasts. In Ink each step is lighter than the one below; never put a darker
surface on a lighter one.

| Role | Light | Ink | Use |
| --- | --- | --- | --- |
| `background` | `oklch(0.975 0.004 250)` | `oklch(0.155 0.018 257)` | Canvas and sidebar |
| `card` | `oklch(1 0 0)` | `oklch(0.19 0.02 257)` | Main panel, cards, inputs, tables |
| `popover` | `oklch(1 0 0)` | `oklch(0.225 0.022 257)` | Menus, dialogs, palette, toasts |
| `elevated` | `oklch(1 0 0)` | `oklch(0.29 0.022 257)` | Raised thumbs and indicators |
| `muted` / `secondary` | `oklch(0.955 0.005 250)` | `oklch(0.25 0.02 257)` | Quiet wells |
| `accent` | `oklch(0.945 0.006 250)` | `oklch(0.27 0.022 257)` | Hover and highlight fill |
| `foreground` | `oklch(0.21 0.034 257)` | `oklch(0.96 0.006 250)` | Primary text |
| `foreground-secondary` | `oklch(0.38 0.03 257)` | `oklch(0.84 0.012 250)` | Supporting text |
| `muted-foreground` | `oklch(0.5 0.022 257)` | `oklch(0.7 0.016 255)` | Metadata, placeholders, resting icons |
| `faint-foreground` | `oklch(0.66 0.015 257)` | `oklch(0.52 0.016 255)` | Disabled text, separators, line numbers only |
| `primary` | `oklch(0.21 0.034 257)` | `oklch(0.96 0.006 250)` | Primary buttons and tooltips (ink, never jade) |
| `border` | `oklch(0.915 0.006 250)` | `oklch(0.28 0.02 257)` | Hairlines and card outlines |
| `border-strong` | `oklch(0.84 0.01 255)` | `oklch(0.36 0.02 257)` | Hover edges, pipeline edges, dot grid |
| `input` | `oklch(0.62 0.016 257)` | `oklch(0.5 0.02 257)` | Outline of anything typed into or toggled |
| `ring` | = `brand` | = `brand` | Focus |

`brand` is jade and means AI: Ask, learning, live processing, checked controls,
focus and the active navigation icon. Keep it rare; more than two brand-filled
elements on one screen means one is wrong.

| Role | Light | Ink |
| --- | --- | --- |
| `brand` (= `success`) | `oklch(0.52 0.095 167)` | `oklch(0.8 0.12 167)` |
| `brand-soft` / `brand-text` | `oklch(0.955 0.03 167)` / `oklch(0.46 0.09 167)` | `oklch(0.29 0.05 167)` / `oklch(0.82 0.11 167)` |
| `warning` / `warning-soft` / `warning-text` | `oklch(0.8 0.14 78)` / `oklch(0.965 0.045 85)` / `oklch(0.48 0.1 65)` | `oklch(0.82 0.13 80)` / `oklch(0.3 0.05 75)` / `oklch(0.86 0.11 85)` |
| `destructive` / `destructive-soft` / `destructive-text` | `oklch(0.54 0.19 27)` / `oklch(0.96 0.025 27)` / `oklch(0.5 0.18 27)` | `oklch(0.68 0.17 25)` / `oklch(0.29 0.07 25)` / `oklch(0.76 0.14 25)` |
| `info` / `info-soft` | `oklch(0.52 0.13 255)` / `oklch(0.955 0.025 255)` | `oklch(0.75 0.11 255)` / `oklch(0.29 0.05 255)` |

Every status pairs its colour with a word and its StatusBadge icon.

### Memory kinds

Each kind owns a text colour and a soft fill tuned to one lightness so no kind
shouts. Use them only to name a kind; `chart-1` to `chart-5` reuse claim,
decision, procedure, handover and entity for series, and the graph theme
adapter uses the same roles.

| Kind | Light text / soft | Ink text / soft |
| --- | --- | --- |
| `kind-claim` | `oklch(0.5 0.13 255)` / `oklch(0.955 0.025 255)` | `oklch(0.8 0.1 255)` / `oklch(0.28 0.05 255)` |
| `kind-decision` | `oklch(0.5 0.1 65)` / `oklch(0.96 0.04 80)` | `oklch(0.83 0.1 80)` / `oklch(0.29 0.045 75)` |
| `kind-procedure` | `oklch(0.5 0.15 300)` / `oklch(0.955 0.03 300)` | `oklch(0.8 0.1 300)` / `oklch(0.28 0.05 300)` |
| `kind-handover` | `oklch(0.5 0.085 200)` / `oklch(0.955 0.025 200)` | `oklch(0.8 0.08 200)` / `oklch(0.28 0.04 200)` |
| `kind-source` | `oklch(0.5 0.09 167)` / `oklch(0.955 0.03 167)` | `oklch(0.8 0.09 167)` / `oklch(0.28 0.045 167)` |
| `kind-entity` (was graph-node) | `oklch(0.5 0.11 145)` / `oklch(0.955 0.035 145)` | `oklch(0.8 0.1 145)` / `oklch(0.28 0.045 145)` |
| `kind-tool` (was tool-call) | `oklch(0.5 0.16 350)` / `oklch(0.955 0.03 350)` | `oklch(0.8 0.11 350)` / `oklch(0.28 0.05 350)` |

### Terminal and brand constants

Agent output sits on a fixed dark surface in both themes: `terminal`
`oklch(0.17 0.02 257)`, `terminal-bar` `oklch(0.21 0.022 257)`,
`terminal-border` `oklch(0.28 0.02 257)`, `terminal-foreground`
`oklch(0.92 0.006 250)` and `terminal-muted` `oklch(0.68 0.016 255)`, with
`ansi-red`, `-green`, `-yellow`, `-blue`, `-magenta` and `-cyan` tuned for it.
The card has a bar with host, command and status, and no window traffic lights.

`brand-ink` `#101b2a`, `brand-jade` `#3d6b59`, `brand-sage` `#8fb7a2`,
`brand-amber` `#e8b861` and `brand-paper` `#f7f4ec` belong to the logo, its
app-icon tile and the cover only. In the product the mark sits on its
`brand-paper` tile (28px in the sidebar) so its original colours read in both
themes. The wordmark is Newsreader 600 with a `brand-text` full stop. Keep the
original SVG mark, monochrome symbol and portable logo in `web/public/brand/`.

## Typography

| Role | Family | Size / line height |
| --- | --- | --- |
| `title-1` page title | Mona Sans 600, tight tracking | 28 / 34 |
| `title-2` section | Mona Sans 600 | 20 / 28 |
| `title-3` card and dialog | Mona Sans 600 | 16 / 24 |
| `body` (default), `body-medium` (nav, primary cells) | Mona Sans 400 / 500 | 14 / 20 |
| `label` buttons and fields | Mona Sans 500 | 13 / 18 |
| `caption` | Mona Sans 400 | 12 / 16 |
| `body-lg` docs prose and composer | Mona Sans 400 | 16 / 26 |
| `display` marketing and docs heroes only | Mona Sans 600 | 40 / 44 |
| `answer` Ask answers and quoted memory | Newsreader 400 | 17 / 28 |
| `hero` Ask greeting | Newsreader 400 | 40 / 46 |
| `wordmark` | Newsreader 600 | 20 / 24 |
| `code` | JetBrains Mono 400 | 13 / 20 |
| `mono-sm` identifiers, times, numerals (tabular) | JetBrains Mono 400 | 12 / 18 |
| `mono-label` pipeline stages, inspector section labels | JetBrains Mono 500, uppercase, 0.06em | 11 / 16 |

Newsreader never sets interface chrome. Headings use `text-wrap: balance`; prose
caps at `reading-measure` (68ch). Use relative units, browser zoom and
selectable text.

## Space, shape, depth and layout

- Space: a 4px scale (4, 8, 12, 16, 20, 24, 32, 40, 48, 64px).
- Radii: 4px checkboxes, kbd and citations; 6px menu items, chips, tooltips;
  8px buttons, inputs, nav items; 12px cards, menus, the panel, pipeline stages;
  16px dialogs, palette, composer; full for badges, avatars, switches, progress.
- Shadows say how far something floats: `xs` secondary buttons and inputs, `sm`
  the panel and hovered cards, `md` menus and tooltips, `lg` dialogs, palette
  and toasts. Ink adds a 1px top highlight. `shadow-glow` appears only while
  Recollect is working. Translucent blur is for the sticky topbar and table
  headers only; scrims use `overlay` with `blur-overlay`. No glass cards.
- Shell: the 240px sidebar (56px rail when collapsed, toggled with ⌘\) sits on
  `background`; content lives in one inset `card` panel, 8px from the window
  edges, with `radius-lg`, a border and `shadow-sm`, under a sticky 52px topbar.
  Content pads 24px (32px from 1728px), caps at 1200px for reading and 1600px
  for tables; Graph and pipeline canvases are full-bleed and controls never
  stack above a canvas.
- Inspector: one record at a time, docked at 400px inside the panel from 1440px,
  a sheet with `shadow-lg` and the scrim below that; focus returns to the row
  that opened it.
- Command palette: 640px, ⌘K, grouped results (at most eight per group), Ask
  hand-off as the first row, no destructive actions.
- Composer and answers: 720px. Dialogs: 480px.
- Density: controls 32px (28 compact, 40 large), table rows 44px, nav items 32px.
- Screens: 1440px is the smallest supported layout. Review at 1440 × 900,
  1536 × 864, 1728 × 1117 and 2560 × 1440 in both themes. No mobile design scope.

## States

Hover fills with `accent`; secondary buttons and cards move to `border-strong`.
Selected rows take a soft brand mix; the active nav item is a raised
`sidebar-accent` chip with a brand icon (never a green-tinted row); tabs and
segments use one sliding indicator. Focus is a 2px `ring` outline at 2px offset
everywhere, and inputs add the focus halo. Disabled uses 0.5 opacity and keeps
its label. Waiting pipeline stages use 0.72 opacity and a dashed border. Every
read has content, shaped loading, empty-with-next-step and error-with-retry
states.

## Motion

| Token | Value | Curve | Use |
| --- | --- | --- | --- |
| `motion-instant` | 100ms | standard | Press feedback, tooltip exit |
| `motion-fast` | 160ms | standard | Hover, colour, border, menus, toggles |
| `motion-base` | 220ms | out | Entrances, dialogs, toasts, tab indicators |
| `motion-slow` | 360ms | out | Inspector, layout reflow, answer reveal, progress, graph refocus |
| `motion-ambient` | 1600ms | standard or linear | Loops only: live ping, AI shimmer, skeleton sweep, edge flow |
| `stagger-step` | 30ms | n/a | First-load list entrance, at most 10 steps |

Easings: standard `cubic-bezier(0.2, 0, 0, 1)`, out
`cubic-bezier(0.16, 1, 0.3, 1)`, in `cubic-bezier(0.4, 0, 1, 1)`, and bounce
`cubic-bezier(0.34, 1.36, 0.64, 1)` for small controls only. Exits use `in` at
`motion-fast`. Wrap the app in `MotionConfig reducedMotion="user"`; animate
transform, opacity and filter, and use layout animations for size and position.

- AI states: a shimmer line while Ask gathers evidence (never a spinner in the
  answer area); a validated answer revealed whole from 4px blur, citations after
  it; a brand border and glow on anything actively processing, removed as soon
  as the server reports a change; new learned rows arrive with a brief
  `brand-soft` highlight and counters roll with tabular digits.
- Pipelines: stages stay in place while their state changes; completed edges
  turn brand at half opacity; the edge feeding a running stage flows; one packet
  per recorded event and none without events; progress eases to each new
  reading and never runs ahead of the server. Motion interpolates between the
  contracted feed readings; the design's 3-second polling example does not
  change the desktop contract's refresh cadence.
- Interface: pages fade and rise 4px through TanStack Router view transitions
  while the sidebar and topbar stay still; lists stagger on first load only;
  menus pop from 0.96, dialogs and the palette rise 8px from 0.98, the inspector
  slides 16px from the right; selection moves with a shared indicator; hover
  never moves layout.
- Never: animated gradients, particles, parallax, bounce on anything larger than
  a control, or looping motion on an idle screen. Every frame reads correctly
  with motion off.

## Iconography

Lucide only, outline at 1.75 stroke: 16px for buttons, nav, menus and rows; 14px
for badges and dense cells; 20px for empty states and tiles; `currentColor`,
`muted-foreground` at rest. Sections: Dashboard `layout-dashboard`, Ask
`message-square`, Explore `compass`, Memory `notebook-text`, Sources `database`,
Graph `network`, Repositories `folder-git-2`, Agents `bot`, Connections
`unplug`, Activity `history`, Settings `settings-2`, Brains `boxes`, Team
`users`. `sparkles` marks Ask and learning only. States: `circle-check` done,
`loader-circle` running, `clock-3` waiting, `circle-alert` failed,
`circle-dashed` unchecked. Icon-only controls have an accessible name and a
tooltip. `web/src/app/navigation.ts` stays the single navigation map.

## Components and implementation

The 31 component specifications are Alert, AnswerCard, AppShell, AskComposer,
AsyncState, Badge, Brand, Button, Card, CategoryChip, CodeBlock, CommandPalette,
Field, IconButton, Inspector, Kbd, Menu, Modal, PageHeader, PipelineFlow,
SegmentedControl, Sidebar, StatusBadge, StatusDot, Table, Tabs, TerminalCard,
Toast, Toggle, Tooltip and Topbar. Each starts from its shadcn/ui counterpart
(Sonner for toasts, cmdk `command` for the palette, `resizable` and `sheet` for
the inspector, `sidebar` in its inset variant, React Flow for PipelineFlow) and
is edited in place in the repository. Mantine pieces with no direct counterpart
— number input, multi-select, JSON input, stepper, copy and file buttons — are
rebuilt from those primitives. FilterBar, ScopeSummary, SettingsSection and
ActionMenu remain Recollect compositions over them. Prefer focused composition
to one universal component with many flags.

PageHeader is a compact sans `title-1` header with an optional one-sentence
description and at most two actions; it never has an eyebrow. Only Ask uses the
serif `hero`, centred above the composer.

TanStack Router provides parent layouts and `Outlet`. Each Brain page is a real
child route with validated non-sensitive URL state, including inspector
selection. Keep questions, answers, secrets and private paths out of URLs.
Lazy-load active routes, share existing typed API hooks, and preserve required
visible-content validity polling. Keep the generated OpenAPI types and canonical
Rust mutation and authorization handlers. Node remains build tooling.

## Fonts

Self-host Mona Sans (variable), JetBrains Mono and Newsreader from `google/fonts`
at a pinned revision, with sha256 entries in `web/public/fonts/manifest.json`
and OFL notices in `web/public/licenses/`. The public documentation site copies
the same three families and notices from `web/public/`; Manrope and DM Mono were
removed on 2026-10-20. Load only needed weights, use explicit
fallbacks and `font-display`, and test offline and missing fonts. The design
system's previews load fonts from Google Fonts; the product must not.
[Mona Sans OFL](https://github.com/google/fonts/blob/main/ofl/monasans/OFL.txt),
[JetBrains Mono OFL](https://github.com/google/fonts/blob/main/ofl/jetbrainsmono/OFL.txt),
[Newsreader OFL](https://github.com/google/fonts/blob/main/ofl/newsreader/OFL.txt).

## Public site and README

The [ADR 0025 amendment](../adr/0025-two-theme-desktop-redesign.md#amendment-2026-10-20-public-site-and-readme)
extends this system to `recollecthub.com` and the README; the
[site contract](../contracts/public-product-and-docs-site.md) carries the
testable behavior.

- The VitePress theme imports `web/src/design/tokens.css` and maps its variables
  onto these roles, so the site has no colour values of its own. VitePress's
  theme switch drives `data-theme` before first paint.
- Docs prose uses `body-lg` within a 760px column; page titles are Mona Sans 650
  at 32/38 and sections 22/30, both with tight tracking. The product page's hero
  uses `display`, scaled up to 54px on wide screens. Sidebar group labels use
  `mono-label`; the current page is a raised `sidebar-accent` chip.
- Links in prose use `brand-text`; primary buttons stay ink. Code blocks sit on
  `muted` with dual GitHub syntax themes; shell walkthroughs and illustrated agent
  output use the `terminal` surface in both themes.
- Explanatory diagrams follow the pipeline rules above (still rest edges, brand
  completed edges at half opacity, a flowing edge into the live stage, one packet
  per illustrated event, glow only on the live element). They replay only while
  visible, with a pause control and step chips, and stop at a complete still
  frame under reduced motion.
- README illustrations are generated by `scripts/render-readme-art.mjs` as
  Light and Ink SVG pairs with literal hex values converted from these tokens,
  because GitHub shows SVGs as images. They use system font stacks, SMIL and CSS
  motion, and hide packets under reduced motion.

## Contrast

WCAG ratios calculated on 2026-10-19 from the OKLCH tokens (converted to sRGB):

| Foreground / background | Light | Ink |
| --- | --- | --- |
| `foreground` / `background`, `card` | 16.49, 17.72 | 17.41, 16.45 |
| `foreground-secondary` / `card`, `muted` | 10.00, 8.78 | 11.32, 9.80 |
| `muted-foreground` / lowest of all surfaces | 5.11 (`accent`) | 5.29 (`elevated`) |
| `faint-foreground` / `card` | 3.11 | 3.36 |
| `input` / `card`, `background` | 3.64, 3.39 | 3.08, 3.26 |
| `ring` (= `brand`) / `card` | 5.24 | 10.40 |
| `brand-text` / `brand-soft` | 6.02 | 8.28 |
| `destructive-text` / `destructive-soft` | 5.77 | 6.38 |
| `warning-text` / `warning-soft` | 6.03 | 8.93 |
| Lowest `kind-*` / its soft fill | 5.07 | 7.60 |
| Lowest terminal text (`terminal-muted`) / `terminal` | 6.64 | 6.64 |

Text roles clear 4.5:1 and `input` and `ring` clear 3:1 in both themes.
`faint-foreground` and `border-strong` (about 1.7:1) are unsuitable for
ordinary text. These calculations are not an accessibility conformance claim:
validate rendered combinations, persistent labels, keyboard access, focus
return, reduced motion, error recovery and the graph's list alternative.
[W3C text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html),
[non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html),
[focus visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html).

## Token enforcement and acceptance

Tokens and the theme come first, then the shell and pages. Since
`desktop-tailwind-design-check` shipped, `web/scripts/check-design.mjs`:

- fails on arbitrary colour values in class names (`-[#…]`, `-[rgb…]`,
  `-[hsl…]`, `-[oklch…]`, `-[color:…]`);
- fails on colour utilities that do not name a token role, including Tailwind's
  default palette (`bg-red-500`) and bare `white`/`black` (side-border width
  utilities such as `border-b` are not colours);
- fails on inline `style` colours and on raw colour literals, including
  `oklch(…)`, outside the design layer;
- checks the Mona Sans, JetBrains Mono and Newsreader manifest entries and OFL
  notices, and keeps the SVG identity checks.

Keep graph geometry and genuine domain layout exceptions explicit, and do not
rely on rules that confuse source content strings with style declarations.
Verify every direct route, active state, browser history, role loss, Brain
switch, both themes, offline fonts, component states, keyboard interaction and
the review sizes. Mockup or preview review alone is not functional, runtime or
accessibility acceptance.

## Dependency verification

| Source | Lookup evidence, 2026-10-19 |
| --- | --- |
| Context7 `/websites/tailwindcss` | `--color-*: initial` removes the default palette (`bg-red-500` no longer exists); `@theme inline` maps external CSS variables that change per theme |
| Context7 `/shadcn-ui/ui` | Vite install with `@tailwindcss/vite`; components read OKLCH `--background`, `--card`, `--primary`, `--sidebar-*`, `--chart-*` variables through `@theme inline` |
| `google/fonts` | `ofl/monasans`, `ofl/jetbrainsmono` and `ofl/newsreader` OFL notices resolve (HTTP 200) |

Version 1 recorded Mantine and TanStack lookups from 2026-09-26; see the pinned
earlier version. These are supported-interface observations, not proof that the
migrated application builds, performs or passes accessibility review.

## Management layouts

The [accepted management amendments](../contracts/desktop-experience.md#approved-management-concepts--2026-10-05)
and their six concept images guide the composition of Connections, setup,
global Connectors, Privacy, AI permissions and Agents; version 2 restyles them
without moving their content. Keep global navigation directly above the Brain
selector. Connection rows and a selected inspector share the same top edge;
primary actions stay visible at laptop height. Broad setup forms use a
persistent action footer and scrollable fields. Policy labels retain their row
position between read and edit. AI purposes and content permissions remain
visible, with actual coverage exceptions emphasized; dimensions, histories and
provider checks belong in explicit diagnostics. Agent setup and memory-read
checks foreground copy controls with optional full prompts. Independent review
compares the shipped page with the composition of the approved concepts and the
version 2 styling, including long forms, empty and filtered states and Cancel.
