# Motion design foundation: warm dark theme, motion tokens and shared primitives

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Replace the light cream desktop theme with the directed warm dark visual language (canvas near-black warm, elevated surfaces, per-category glow accents) and introduce a coherent motion system — durations, easings, entrance stagger, pulsing status, skeleton loaders, page transitions — as shared primitives applied to the core shell.
- Non-goals: No new pages, data feeds, routes or permissions; no per-page feature logic changes; no pipeline view, terminal cards or ambient mode (later slices); no mobile layout work; no new animation library dependencies.
- Delivery shape: Web design-token + primitive changes, design-system document update, contract amendment already recorded, live browser verification.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md) (amended 2026-10-03: visual language and motion section)
- [Design system document](../../desktop-experience/design-system.md) (updated by this pack)
- [claude-mem study, 2026-10-03](../../../research/claude-mem-study-2026-10-03.md) (warm-dark token reference: base #1a1916 / cards #252320, per-category accents with low-alpha tints)
- [Owning epic](../../epics/desktop-visual-experience.md)

## Scope

- In scope: New warm dark token layer in `web/src/design/tokens.ts` and CSS custom properties — canvas, surface levels (2–3), ink/secondary/muted text, borders, accent (amber-orange primary), violet secondary, and per-category accents for memory kinds (claim, decision, procedure, handover) plus source/graph/tool-call categories; each with a low-alpha tint for badges/glows. Motion tokens: durations (fast ~150ms, base ~250ms, slow ~450ms, ambient ~1200ms), easings (standard, emphasized/decelerate for entrances), stagger step ~40ms. Shared primitives in `web/src/components/`: entrance animation utility (fade+rise with stagger for lists/cards), pulsing status dot (processing/live), skeleton loader block, and a page-content transition wrapper. Apply to the core shell: app sidebar/header, PageHeader, AsyncState loading/empty/error, card/list rows on Memory, Sources, Agents (per-Brain and global), Activity, Connections and Settings. Honor `prefers-reduced-motion` by disabling transform/opacity animations (content still appears). Update the design-system document with the new palette, type scale confirmations and motion rules.
- Out of scope: Pipeline visualization, terminal cards, ambient mode; any backend change; changing which data a page shows; auth/OIDC view content (they inherit tokens only); font changes (Newsreader/Manrope/DM Mono stay).
- Blockers: None. The 2026-10-03 user direction (cmem.ai-style modern animated desktop look, desktop-first) is the governing decision; the contract amendment lands in this change set.

## Surface and Interface Changes

- Interfaces: None — presentation only over existing reads.
- Storage: None.
- Ownership: The design token layer and motion primitives are shared shell concerns; pages keep owning their content.

## Data and Authority

- Inputs: Unchanged page data.
- Authority: Unchanged; theme and motion grant nothing.
- Blind spots: Dark theme raises the cost of low-contrast text — every semantic role (ink, secondary, muted, border, accent) must meet readable contrast on dark surfaces; status meanings stay non-color (icons/labels retained).

## States and Edge Cases

- Loading: Skeleton loaders replace spinners in list/card regions; shell chrome animates in once.
- Empty: Empty states keep their copy and icons; they fade/rise in like content.
- Error: Error states keep red semantics on the dark surface with readable contrast.
- Blocked: No new blocked state; existing gating unchanged.
- No-access: Standard access rules apply before any visual logic.
- Duplicate or replay: Pure presentation; no writes, no replay concerns.
- Stale data: Unchanged polling cadences; animations never mask staleness (no "live" claims without a processing feed).
- Reconciliation divergence: None — no new data path.

## Integrations and Runtime Inputs

- Providers: None.
- Environment: None.
- Secrets: None.
- Failure handling: `prefers-reduced-motion` and missing CSS variables degrade to static content; no JS animation engine to fail.

## Tests and Acceptance

- Automated: Web typecheck and design checks (`npm run typecheck` in web/).
- Manual: Owner login at the live stack — shell, Memory, Sources, Agents (per-Brain + global), Activity, Connections, Settings all render on the warm dark theme with readable contrast; list rows stagger in; loading shows skeletons; a processing status pulses where one exists; page switches transition smoothly; with `prefers-reduced-motion` emulated, content appears without motion.
- Acceptance: Browser verification passes on real data at 1280×800 and 1920×1080; typecheck and design checks clean; `./scripts/validate.sh` passes; live stack rebuilt via `./scripts/stack.sh up --build` with api/worker healthy.

## Closeout

- Planned: Warm dark tokens, motion tokens, shared primitives, shell application, design-system doc update, browser proof.
- Shipped:
  - Warm dark token layer in `web/src/design/tokens.ts`: canvas `#141310`, surfaces `#1c1a17`/`#252320`, ink `#f2efe9`, secondary `#a8a29e`, borders `#2e2b26`, amber-orange primary `#f97316`, violet secondary, per-category accents (claim blue, decision gold, procedure violet, handover cyan) plus source/graph-node/tool-call categories, each with a ~12–16% alpha tint. Same export shape preserved.
  - Motion tokens in the same layer: fast 150ms / base 250ms / slow 450ms / ambient 1200ms, standard + emphasized easings, 40ms stagger step (capped at 12 steps so long lists settle by ~480ms).
  - `web/src/design/theme.ts`: Mantine category color scales and `categoryRole()` variant mapping, amber filled primary buttons, motion values and `--rc-cat-*` CSS variables in the resolver.
  - `web/src/styles.css`: keyframes (`rc-rise`, `rc-fade`, `rc-pulse-ring`, `rc-shimmer`), `.rc-enter`, `.rc-page-enter`, `.rc-pulse-dot(.live)`, `.rc-skeleton(-rows)`, tokenized transitions, and a strengthened `prefers-reduced-motion` block that zeroes transition/animation durations *and* delays so `fill-mode: both` cannot hold content hidden.
  - Shared primitives in `web/src/components/`: `Motion.tsx` (`staggerStyle` + `Stagger`), `StatusDot.tsx`, `Skeleton.tsx` (`Skeleton` + `SkeletonRows`), `PageTransition.tsx`.
  - Applied to the core shell: `App.tsx` (page-transition wrapper on route content, pulsing service-status dot in the topbar, entrance on sidebar/topbar), `AsyncState` (skeleton loading, fade-in empty/error), `PageHeader`, `SettingsSection`, and list/card regions on Memory (`ClaimsPanel`, `EvidencePanel`, `HandoversPanel`), Sources, Agents per-Brain (`AgentsRoster`) and global (`GlobalPages`), Activity (`ActivityPage`, `JobsPanel`), Connections (`McpPanel`), Settings, and the Brains home grid (`BrainsPage`).
  - `web/index.html` theme-color set to `#141310`; the three brand SVGs recolored (dark ink → light ink + amber) because their dark strokes were invisible on the dark sidebar; assets remain self-contained with titles, so the design check still passes.
  - Browser proof (owner login, live stack at http://127.0.0.1:8787, 1440×900): screenshots `motion-brains-home.png`, `motion-brain-ask.png`, `motion-memory.png` (staggered rows with claim/procedure/handover category badges), `motion-sources.png`, `motion-agents-brain.png` (empty state), `motion-agents-global.png` (status dots on 12 agents), `motion-activity.png`, `motion-activity-processing.png`, `motion-connections.png`, `motion-settings.png` in `/private/var/folders/j6/6lx_5zxs5377bhb8fq09pvrc0000gn/T/opencode/`. All surfaces render on the warm dark theme with readable contrast; console clean (no errors or warnings).
  - Deviations, recorded: violet secondary brightened `#8b5cf6` → `#9b7ff5` so badge text on its tint meets WCAG AA (4.82:1 vs 3.54:1); category hues chosen where the pack left them open — source = primary amber `#f97316`, graph-node = green `#4ade80`, tool-call = pink `#f472b6`; brand SVG recolor (allowed by scope: "adjust if invisible on dark"); `check-design.mjs` needed no change (it asserts structure — shared colors/fonts, four pinned fonts + licenses, self-contained SVGs — not specific values).
- Not shipped: Nothing in scope. Pipeline view, terminal cards and ambient mode remain later slices per the epic; mobile layout work is out of scope.
- New blockers: None.
- Docs updated: `docs/roadmap/desktop-experience/design-system.md` — warm dark palette section (canvas/surfaces/ink/borders/accent values), category accent table with tint alphas, motion system section (durations, easings, stagger, primitive classes, reduced-motion rule), and refreshed contrast ratios.
- Validation:
  - `cd web && npm run typecheck` — PASS (includes the design check).
  - `./scripts/validate.sh` — PASS.
  - `./scripts/stack.sh up --build` — postgres/neo4j/worker/api all Healthy; app served at http://127.0.0.1:8787.
  - Browser verification with owner login: Brains home, Ask, Memory, Sources, Agents (per-Brain + global), Activity (overview + processing), Connections and Settings all render on the warm dark theme with readable contrast; list rows stagger in; skeletons replace spinners in list/card regions; status dots pulse where live; page switches transition via the wrapper; brand logo visible in the sidebar. Console: no messages.
  - `prefers-reduced-motion`: verified at the CSS level — the media block zeroes all transition/animation durations and delays, so content appears instantly and fully; browser-level emulation was not available with the verification tooling.
- Version: N/A: no release policy or version bump in this scope.
- Commit: Uncommitted; no commit, push or release performed.
