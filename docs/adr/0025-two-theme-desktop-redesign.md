# 0025: Two-theme desktop redesign on Tailwind and shadcn/ui

Status: accepted

## Decision

On 2026-10-19 the user approved version 2 of the Recollect design system, a
2026 redesign of the desktop interface, and asked for it to be implemented. The
[design system foundation](../foundation/design-system.md) records its tokens,
type, layout, components and motion; this record decides the architecture.

- **Stack.** React 19, TypeScript, Vite, TanStack Router and Query,
  openapi-fetch and Playwright stay. Mantine and the hand-written feature CSS
  are replaced by Tailwind CSS v4 and shadcn/ui components copied into the
  repository, built on one primitive family (Radix or Base UI, chosen once at
  implementation). Motion handles shared layout and transitions, and React Flow
  draws bounded read-only diagrams such as the pipeline. The knowledge graph
  keeps the renderers [ADR 0021](0021-grouped-spatial-graph-navigation.md)
  selected, themed from the same tokens. Lucide stays the only icon set.
- **Themes.** Two themes, Light and Ink (dark), designed with equal care,
  replace the single light-only token system. The theme follows the system
  setting until a person chooses one, the choice is remembered, and the
  `data-theme` attribute is set before first paint. Tokens are OKLCH values
  under shadcn's variable names (`background`, `foreground`, `card`, `popover`,
  `primary`, `muted`, `accent`, `border`, `input`, `ring`, `chart-*`,
  `sidebar-*`), plus Recollect roles for `brand`, status, memory kinds,
  terminal and ANSI. Jade `brand` is reserved for AI and learning states: Ask,
  learning, live processing, checked controls, focus and the active navigation
  icon. Primary actions are ink, never jade.
- **Type.** Mona Sans sets the interface, JetBrains Mono sets machine text
  (identifiers, times, counts, code) and Newsreader is used only for answers,
  the Ask hero and the wordmark. All three are self-hosted with manifest hashes
  and SIL OFL notices; the product makes no runtime external font request.
  Manrope, DM Mono and serif page and section headings are retired.
- **Layout.** 1440px is the smallest supported layout, for 16-inch laptops and
  larger. The sidebar sits directly on the canvas and content lives in one
  inset main panel. A selected record opens in a docked 400px inspector inside
  that panel; below 1440px the inspector opens as a sheet and the sidebar
  collapses to its rail. A ⌘K command palette searches, navigates, runs safe
  actions and hands a query to Ask.
- **Page silhouette.** The serif Ask hero (Newsreader, centred, above the
  composer) replaces [ADR 0017](0017-desktop-knowledge-and-ask-experience.md)'s
  "eyebrow + large serif heading" silhouette reserved for Ask. Every other page
  uses a compact sans page header without an eyebrow.

Routes, data ownership, permissions, the read-only answer posture and every
existing failure state are unchanged.

## Why

The user asked for a modern 2026 redesign and approved the resulting design
system ("feel free to decide by yourself"). Version 1 of that system
documented the shipped Mantine, cream and sage interface; version 2 is the
approved target. The redesign artefact is the
[Recollect design system](https://claude.ai/artifact/MpZJ2m2L2D5GkNhgQfeFV6),
version 2 (published 2026-10-19 as version `1792413783-6f3b`), whose
`project/README.md`, `project/motion.md`, `project/implementation.md` and
`project/tokens.json` were read for this record.

shadcn/ui reads exactly the variable names chosen for the tokens, so one token
stylesheet themes the copied components and Tailwind's utilities together, and
OKLCH keeps both themes perceptually even. A dark theme and a docked inspector
are expected in current desktop tools, and the 16-inch floor lets the inspector
dock beside content instead of covering it. Today Mantine's theme and the
hand-written feature CSS are two styling layers over one token set; a single
token stylesheet read by both the copied components and the utilities removes
that split.

Context7 lookups on 2026-10-19 (`/websites/tailwindcss`, `/shadcn-ui/ui`)
confirmed that Tailwind v4 removes its default palette with `--color-*: initial`,
maps external variables with `@theme inline`, and that shadcn/ui's Vite install
and theming read OKLCH CSS variables under these names. The OFL notices for
Mona Sans, JetBrains Mono and Newsreader resolve in `google/fonts` on the same
date. These are interface observations, not proof that the migrated
application builds, performs or meets contrast in either theme.

Alternatives considered: restyling Mantine in two themes, which keeps the
parallel CSS layer and fights the shell; and keeping the cream light theme with
new fonts only, which the user's request for a modern redesign with a dark
theme rules out.

## Consequences

The [desktop contract](../contracts/desktop-experience.md) and the
[knowledge surface contract](../contracts/desktop-knowledge-surface.md) carry
the testable layout, theme, typography, inspector and palette behavior. The
[technology stack](../foundation/techstack.md) names the new browser stack.
Implementation is sequenced in the
[desktop visual experience epic](../roadmap/epics/desktop-visual-experience.md).

- Feature code uses only token-backed utilities. `web/scripts/check-design.mjs`
  must change before the migration can pass: it fails on arbitrary colour
  values in class names (`-[#…]`, `-[rgb…]`, `-[hsl…]`, `-[oklch…]`), on colour
  utilities outside the token theme (Tailwind's default palette, such as
  `bg-red-500`, does not compile), and on inline `style` colours; and it checks
  the Mona Sans, JetBrains Mono and Newsreader manifest entries and OFL
  notices in place of Manrope and DM Mono.
- Both themes must meet the existing contrast, focus, reduced-motion and
  non-colour status obligations. The terminal surface is dark in both themes.
- The migration replaces every Mantine component in place, including those
  with no direct shadcn counterpart (number input, multi-select, JSON input,
  stepper, copy and file buttons). No route, URL state, API call, permission,
  polling cadence or failure state may change as a side effect; existing
  Playwright and domain regressions keep their meaning.
- Brand constants (`brand-ink`, `brand-jade`, `brand-sage`, `brand-amber`,
  `brand-paper`) belong to the logo and its app-icon tile only. The original
  SVG mark is kept.
- The public documentation site is outside this decision. Its
  [contract](../contracts/public-product-and-docs-site.md) keeps the cream, ink
  and sage system until the user decides whether `site/` adopts these themes.
- Acceptance of the design is not evidence of delivery. Delivery needs real
  browser proof in both themes at the review sizes in the desktop contract.

## Supersession

Amends [ADR 0014](0014-desktop-experience-and-answers.md): replaces its Mantine
frontend, its "one light-only token system" and its Newsreader, Manrope and
DM Mono typography. Its routes, Lucide icons, original SVG identity, feature
ownership and evidence-backed read-only Ask remain.

Amends [ADR 0017](0017-desktop-knowledge-and-ask-experience.md): the eyebrow and
large serif heading silhouette reserved for Ask becomes the serif Ask hero, and
other pages use compact sans headers. Its tiers, shared inspector, wiring split
and assurance band remain; the inspector now docks at 400px.

Amends [ADR 0003](0003-product-runtime.md) and
[ADR 0021](0021-grouped-spatial-graph-navigation.md) only where they name
Mantine or the cream, ink and sage palette. The desktop contract's
2026-10-01 display priority is narrowed to the 1440px floor; dark mode leaves
the desktop deferrals.
