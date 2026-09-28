# Accepted desktop design system

Status: accepted
Evidence date: 2026-09-26
Accepted: 2026-09-26, by explicit user approval and implementation request.
Scope: desktop browser UI, light theme only; implementation follows the desktop contract.

The [desktop contract](../../contracts/desktop-experience.md) makes these tokens
and shared visual rules authoritative for the redesign. Add the user's requested
original Recollect SVG mark/wordmark/favicon, inspired by gathering memories again.
Use one Lucide size/stroke convention and shared icon map; generated concept icons
are illustrative and do not justify per-page variants.

## Reference and authority

Keep the accepted [stack](../../foundation/techstack.md): React, TypeScript,
Vite, Mantine, TanStack Query/Router and Rust static same-origin delivery.
The user accepted retaining this stack; no replacement framework is needed.
Atlas is a visual/research reference, not product authority.

The local Atlas checkout was clean at
`7eca7f7abd934c2e44bc096fc1dd0cbf94275b99`.
[Palette](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/assets/main.css), lines 1–21;
[font imports](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/templates/document.html), lines 60–63;
[pinned CSS](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/assets/main.css).
Its [MIT license](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/LICENSE) names Simon Strandgaard.
Preserve the notice when copying substantial source. Recollect retains its name.

Generated concepts do not enforce measured pixels or exact font rendering.
Written tokens and implementation acceptance govern; invented dates, icons,
counts and labels in an image are not API/schema decisions.

## Three token layers

1. Primitives: source colors, families, numeric scales.
2. Semantic roles: canvas/surface/text/border/action/selection/focus/status.
3. Component roles: navigation selection, field outline, graph node/edge styling.

Feature code uses semantic/component tokens. A graph theme adapter uses the
same palette; Cytoscape must not develop a separate color system.

## Exact Atlas palette

| Primitive | Value | Application use |
| --- | --- | --- |
| ink | `#101b2a` | Primary text and main action |
| ink-soft | `#2f3d4f` | Supporting text |
| paper | `#f7f4ec` | Canvas |
| paper-deep | `#eee9de` | Light sidebar/grouping |
| paper-light | `#fffdf8` | Inputs, panels, dialogs |
| line | `#d7d0c2` | Decorative separators |
| line-dark | `#a9a192` | Stronger decorative separators |
| sage | `#8fb7a2` | Muted accent fills |
| sage-dark | `#3d6b59` | Links, selected labels, positive emphasis |
| amber / amber-soft | `#e8b861` / `#f4e2bd` | Attention accents/background |
| rose | `#d98272` | Error accent/background, not small text |
| blue / violet / cyan | `#7f9fc6` / `#a58fbd` / `#7ab5b2` | Restrained graph categories |

Accepted application extensions, not original Atlas primitives:

| Semantic role | Value |
| --- | --- |
| Field boundary | `#857c6b` |
| Muted text | `#59636a` (at least 5:1 on all three paper surfaces) |
| Selection background | `#e8f0e9` |
| Selection foreground | Atlas dark sage |
| Primary action background / foreground | Atlas ink / light paper |
| Danger foreground / background | `#9b3f31` / `#f8e5df` |
| Attention text / background | Atlas ink / soft amber |
| Keyboard focus outline | Atlas dark sage; amber optional secondary accent |

Do not copy the reference site's large navy marketing hero into the app.
The sidebar and main canvas stay light. No dark-mode switch.

## Typography and geometry

Self-host **Newsreader**, **Manrope**, **DM Mono**. Atlas imports Newsreader
500/600 (with optical sizing), Manrope 400/500/600/700, DM Mono 400/500.
Use actual selected weights/styles, not accidental synthetic italics/bold.

| Role | Family | Reference size / line height |
| --- | --- | --- |
| Page title | Newsreader 500 | 36–40px / 1.1 |
| Major section | Newsreader 500 | 24px / 1.2 |
| Component title | Manrope 600 | 16px / 1.4 |
| Reading text | Manrope 400 | 16px / 1.55 |
| Navigation/control/table | Manrope 400–600 | 14px / 1.45 |
| Metadata/identifiers | DM Mono 400 | 12px / 1.5 |
| Navigation group | DM Mono 500 | 11–12px / 1.5 |

Use relative CSS units, browser zoom, selectable text and readable line lengths.
Serif is selective: not every row/label/subtitle becomes editorial display type.

- Space scale: 4, 8, 12, 16, 24, 32, 48px.
- Radii: controls 4px; panels 6px; dialogs 8px; pills for short status only.
- Controls: 40px standard; 32px compact with proper accessible names.
- Shell: 248px sidebar, 64px topbar, 32–40px padding.
- Inspector: 360–420px when open; reading width roughly 68–80ch.
- One-pixel separators; shadows for overlays only.
- Short functional transitions; respect reduced motion.
- Proof sizes: 1280×800, 1440×900/960, 1920×1080/1200. No mobile design scope.

## Shared components and implementation

| Component | Responsibility |
| --- | --- |
| WorkspaceShell / BrainShell | Contextual navigation, account and Brain boundaries |
| PageHeader | Title, useful one-line help, dominant action |
| FilterBar / ScopeSummary | Search, applied criteria, advanced drawer |
| DetailInspector / EvidenceInspector | Exact source/revision, applicability, history, return path |
| StatusBadge / ConnectionStatus | Words and icon with color; configured != successful call |
| AsyncState | Loading/empty/error/forbidden/partial states; preserve safe input, clear invalid protected payload |
| ActionMenu | Rare actions with existing authority and effect |
| SettingsSection | Grouped standing decisions and conditional fields |
| JobStatus | Background outcome/progress/recovery |
| SourceCitation / MemoryRecord | Readable knowledge and inspectable support |

Prefer focused composition to one universal component with many flags.
Build a small development-only component gallery in existing Vite, not another
mandatory documentation framework.

Proposed files:

```text
web/src/design/{tokens.ts,theme.ts,typography.css,graph-theme.ts}
web/src/app/{router.tsx,WorkspaceShell.tsx,BrainShell.tsx}
web/src/components/{layout,feedback,evidence,controls}/
web/src/features/{ask,memory,sources,graph,repositories,agents,connections,activity,settings}/
web/public/{fonts,licenses}/
```

Use Mantine `createTheme`, provider CSS-variable resolver and shared component
extensions. Force light mode; do not scatter colors/radii/fonts through TSX or
target generated Mantine class names. Buttons, inputs, selects, menus, dialogs,
drawers, tabs and tables share defaults. The implemented [theme](../../../web/src/design/theme.ts),
[tokens](../../../web/src/design/tokens.ts) and development-only
[component entry](../../../web/design.html) replace the former thin teal/Inter theme.
The [dated asset evidence](../../mappings/desktop-experience-implementation-2026-09-26.md)
records actual fonts, licenses, SVGs and validation limits.

TanStack Router provides parent layouts and `Outlet`. Each Brain page is a real
child route with validated non-sensitive URL filters. Keep questions, answers,
secrets and private paths out of URLs. Lazy-load active routes, share existing
typed API hooks, and preserve required visible-content validity polling.
Do not mount every panel in a new hidden tab set.

Keep the generated OpenAPI types and canonical Rust mutation/authorization
handlers. Node remains build tooling; no new production Next.js/SSR runtime.

## Dependency verification

The design reviewer successfully used Context7 on 2026-09-26:

| Library ID | Lookup evidence |
| --- | --- |
| `/websites/v8_mantine_dev` | Theme variables, CSS resolver, shared component styles |
| `/websites/tanstack_router` | Nested layouts, Outlet, validated search state |

Official pages also checked:
[Mantine CSS variables](https://v8.mantine.dev/styles/css-variables/),
[MantineProvider](https://v8.mantine.dev/theming/mantine-provider/),
[TanStack routing](https://tanstack.com/router/latest/docs/routing/code-based-routing),
[search parameters](https://tanstack.com/router/latest/docs/guide/search-params),
[Vite static delivery](https://vite.dev/guide/static-deploy).
No unresolved dependency lookup blocks this proposal. These are supported-API
observations, not proof of future implementation compatibility or performance.

## Contrast, focus and fonts

Calculated sRGB ratios from the selected colors:

| Foreground/background | Ratio |
| --- | --- |
| ink / paper | 15.76:1 |
| soft ink / paper | 10.05:1 |
| dark sage / paper | 5.54:1 |
| danger text / paper | 6.07:1 |
| danger text / danger background | 5.48:1 |
| field boundary / paper | 3.75:1 |
| field boundary / light paper | 4.06:1 |
| line-dark / paper | 2.33:1 |
| rose / paper | 2.59:1 |
| amber / paper | 1.67:1 |

The last three are unsuitable for ordinary small text; amber alone is not the
focus outline. Use dark sage, an offset, labels and icons. Validate actual
rendered combinations, persistent field labels, keyboard access, focus return,
reduced motion, error recovery, and the graph's list alternative.
[W3C text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html),
[non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html),
[focus visible](https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html).
These calculations are not an accessibility conformance claim.

Package required fonts locally with their SIL OFL notices:
[Newsreader](https://github.com/google/fonts/blob/main/ofl/newsreader/OFL.txt),
[Manrope](https://github.com/google/fonts/blob/main/ofl/manrope/OFL.txt),
[DM Mono](https://github.com/google/fonts/blob/main/ofl/dmmono/OFL.txt).
Record actual asset versions/licenses during implementation. Load only needed
weights, use explicit fallbacks/font-display, and test offline/missing fonts.

## Competitor/research conclusions

Local Cognee was inspected at
`c0d18c80e24b7b78918e7642c03f6f128fdd2aee`; no Cognee runtime was tested.
Its [navigation](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee-frontend/src/ui/layout/Navbar/CustomAppShellNavbar.tsx)
separates data, exploration and connections. Its
[Search page](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee-frontend/src/app/%28app%29/search/SearchPage.tsx)
(408–413, 637) uses a question input, Brain selector and conversation history.
The [recall client](https://github.com/topoteretes/cognee/blob/c0d18c80e24b7b78918e7642c03f6f128fdd2aee/cognee-frontend/src/modules/datasets/recallKnowledge.ts)
(25–47) defaults to HYBRID_COMPLETION; ordinary users need not choose algorithms.
Its integration UI separates agents, automation and sources.

Official Hindsight documentation distinguishes
[Reflect](https://docs.hindsight.vectorize.io/reflect/) (synthesized cited answers)
from [Recall](https://docs.hindsight.vectorize.io/recall/) (retrieval/debugging).
This supports Ask as the human entry point with an evidence-search inspector.
It does not prove Recollect already implements generated answers.

Atlas patterns
[evidence before belief](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/patterns/evidence-before-belief.md)
and [memory editing](https://github.com/neoneye/agent-memory-atlas/blob/7eca7f7abd934c2e44bc096fc1dd0cbf94275b99/content/patterns/memory-as-an-editing-surface.md)
support keeping sources and effective correction reachable. They are research,
not replacements for Recollect's contracts or a reason to require manual review.

## Token enforcement and acceptance

Establish tokens/theme first, then migrate the shell and capabilities in bounded
slices. Track legacy style exceptions while failing new raw color/font/spacing
declarations outside the token adapter. Keep graph geometry and genuine domain
layout exceptions explicit. Do not rely on grep rules that confuse source
content strings with style declarations.

Verify every direct route, active state, browser history, role loss, Brain
switch, offline fonts, component states, keyboard interaction and desktop width.
Preserve domain regressions while adding a small number of meaningful visual
baselines. Run frontend build/typecheck and repository checks in implementation.
Mockup review alone is not functional, runtime, or accessibility acceptance.
