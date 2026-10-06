# Strict final-reference checklist

Status: target definition and rejection audit. The previous visual acceptance is
superseded by the user's explicit correction. Matching broad structure is insufficient.

Only these final PNGs were inspected: `01-connections.png`, `02-add-connection.png`,
`03-global-connectors-v2.png`, `04-privacy.png`, `05-ai-permissions-v2.png`,
`06-agents.png`, in `output/imagegen/2026-10-05-management-concepts-01a10b36/`.
Rejected real evidence is in `after-comparison/`. The governing slice is
`docs/roadmap/execution/active/desktop-management-vision-fidelity.md`.

Coordinates below are approximate pixel targets read from the final reference
images at 1586 × 992, not DOM measurements. The shared shell varies slightly
between generated references; use a coherent production shell while preserving
each page's composition. Generated records/counts/commands/marks are illustrative.
Do not invent rows, pending entities, credentials or service health to match them.

## Shared visual weight

- Increase ordinary UI icons from the rejected tiny 14–16px treatment to about
  22–24px, with 28–36px glyphs in larger tiles. Use meaningful glyphs for host,
  connector, processing purpose and control instead of the same generic link/robot.
- Navigation labels are about 16–18px; row secondary text 15–18px, rather than the
  rejected 12px. Card names/headings have stronger ink serif weight. Important
  status pills are rounded, legible and about 28–34px high, not thin rectangles.
- Page subtitles are prominent, often serif, around 20–24px. Connections title is
  about 58–60px; Connectors/Agents about 48–50px. Settings around 40–44px.
- Preserve cream/ink/sage, light surfaces, hairline dividers, modest corner radii
  and the actual product mark. Sage primary actions need clear icons and weight.
- Reserve flexible sidebar space for the account footer. Do not reproduce the
  earlier growing gap. Keep global navigation visually distinct from Brain scope.

## Connections — critical differences

- Reference left content: x303, width about 790. Inspector: x1125, y84, width 438,
  bottom around 938. The rejected rail starts y353 and is only359 wide.
- Inspector aligns to the header, not beneath the list/filters. The page title,
  Add connection, tabs, search, environment filter and rows occupy only the left
  pane. Add connection is at about x910/y100, 178 × 46.
- Tabs underline about y255. Search about 438 wide; environment about 242 wide.
  Do not split the entire canvas into two equal filter fields.
- Row tile is about 56 square with a strong semantic/cube glyph. Selected row starts
  y350 and is about 120 high. A rounded status pill and clear Inspect affordance
  occupy the right side; actual data may yield fewer rows.
- Inspector has explicit Configuration, Test status and Tool access sections.
  Show target/copy, scope and runs-on chips; prominent sage Test connection with
  play icon, bordered Edit with pencil, and a tool-group card below access.
  Keep configuration, historical successful calls and current observation truthful.
- The private-network callout belongs inside the left pane at about y819, 790 × 117.
  It must not span beneath the inspector across the whole canvas.

## Add/edit connection — compact structured dialog

- Reference dialog is about 912 wide, x447–1359, y54–951. Rejected dialog is800 wide.
  Use a broad, compact composition and keep its footer exposed at shorter heights.
- Method strip is about 46 high with semantic icons and sage active state.
- Configuration heading is serif; Auto-detect/JSON/TOML/YAML are visible tabs on
  its right, not a select. Keep file import compact, without a tall blank extra row.
- Numbered monospace editor: y276–464, about 188 high. Preserve editable source,
  safe masking and keyboard usability; do not introduce a dependency just for style.
- Parsed result uses a full-width sage success bar, about 37 high. Follow it with
  a compact horizontal server/target, runs-on and environment summary.
- Authentication heading and method strip share a line. Secret variable, masked
  value and actual credential-provider destination are horizontal labeled columns.
  Do not use the rejected full-width secret value under an Authorization heading.
- Optional connection name spans the dialog near y805. Primary footer actions are
  Cancel, Inspect tools and Review connection, aligned right; concise review note left.
  Preserve the established explicit inspection/review/save behavior.
- The screenshot does not define a separate exact edit-form image. Apply its field,
  heading, spacing and footer language consistently to edit without hiding schema,
  enabled state, revision warning, placement or required controls.

## Global Connectors — compact catalogue with prominent import rail

- Main catalogue about x290–1204, width 914. Card columns about 295 wide, with 15–16px
  gaps. A single actual definition should not stretch to the rejected 466px width.
- Header title about 48px; subtitle about 22px. Filled sage Add connector sits in
  the main header around x1014/y98, rather than the far-right edge of the rail.
- Status tabs with actual counts, search about 232 wide and transport filter about 146
  share the compact bar. Render supported states truthfully; do not fabricate pending
  definitions just to copy the illustrative Pending review count.
- Cards use80px pastel icon tiles, serif18–20px names, approval pill, description,
  transport pill and a separate lower View definition action row.
- Import rail: x1230, y76, width 338, bottom around 938. Rejected starts y271 and is
  only300 × 472. Header and large112px import illustration precede copy/actions.
- Filled sage Import definition around 292 × 48; separate bordered Paste configuration
  about 46 high. Numbered circular Import/Review/Approve steps with connecting rule
  and dividers. Do not replace these with one small import button and a plain list.
- Approval/access explanatory callout belongs under the main catalogue, not rail.
  Real sparse inventory is not a defect; composition and visual weight still matter.
- The final reference retains the actually selected Brain picker and scoped sidebar
  navigation below global links. The rejected global route omits this entire lower
  navigation section. Preserve real context if supported safely by the existing shell;
  do not invent it. Any deliberate global-only sidebar must be recorded as a specific
  visual deviation, not excused by the catalogue's sparse inventory.

## Privacy — retention first, stable compact editing

- Retention comes first in one Privacy card: x300/y197, width 1227, height259.
  Privacy + Automatic + description live in its header; the redaction notice sits
  upper right. Do not use the rejected separate floating header and Capture-first order.
- Two raw retention rows about 53 high. Number input about 113 wide, unit about 122,
  both35 high; editing badge beside the relevant row. Display/edit retain geometry.
- Warning strip about 40 high and Cancel/Save changes share the bottom line. Sage
  save action matches the reference; do not put a separate tall action line below.
- Capture card follows at y463, about 217 high: three compact44px rows with policy
  control/toggle presentation, respecting actual edit/disabled authority. Avoid a
  prose summary of Enabled / What may be captured / Included as the main composition.
- Evidence/memory follows around y689. Preserve all separate governing retention
  classes while using compact40–44px rows; do not merge semantics to copy two sample rows.
- Repository text card has References only / Include file text controls on the right
  of the row, with small applicability note. Actual behavior and save rules remain.

## AI permissions — provider/memory strips and three distinct rail cards

- Content starts around y221. Primary card x298, width 834; right rail x1154,
  width 400, gap23. Rejected cards start y271 and the rail is350 wide.
- Primary title AI permissions is32px bold serif. Installed provider is a48px
  bordered horizontal strip with recognizable provider icon/name and configured pill.
- Automatic memory is a separate 66px sage strip with large cycle icon and Enabled
  pill. The rejected tiny AI processing badge cannot substitute for this.
- Allowed processing has its own heading, five roughly 61px rows, 24px purpose icons
  and stacked descriptions. Keep the reference's readable hierarchy rather than the
  rejected inline miniature descriptions. Preserve actual policy flags.
- Allowed content: two columns of compact checked rows around 22px high, beginning
  around y842; no three-column wide31px chips. Privacy link below.
- Search coverage card about 300 high: heading attention pill, two prominent counts,
  blocked/failed count in red, short explanation and Review action. Avoid the large
  warning block and extra prose that made the rejected card439 high.
- Installed models card around y541, height195, with two icon/model/purpose/status
  rows. Do not substitute a provider prose card with a single bottom diagnostics link.
- More options card around y757, height195: separate Diagnostics, Policy history and
  Recent batches rows, each with icon/chevron. Keep all external checks explicitly gated.

## Agents — weighted roster, concise setup and memory cards

- Header My agents belongs below the filled sage Connect agent action, not as a
  lone link beneath the roster. Connect action about 200 × 50 with plus icon.
- Roster owner has64px circular avatar and bold26px name. Three rows around 86px
  high;60px neutral tiles with 32px host glyphs, bold22px ink names and18px subtitles.
  Rejected48px green robot tiles and small green names lack the reference's weight.
- Rounded status pills around 156 × 34 with a green dot. Concise Observed dates around
 16px; preserve exact timestamps through inspection/hover rather than long two-line
  date text. Credential state is not a live connectivity claim.
- Manage my access belongs in the roster footer on the right.
- Setup and memory cards start around y662, widths773/428, gap15; headings around 30px
  bold serif. Host strip46px high with host icons/sage active state.
- Compact real command/prompt block with copy icon and View setup below. Use real
  supported plugin commands; never fabricate the image's illustrative CLI shorthand.
- Memory copy action is a full-width 47px button. Private-context link below; readable
 18px explanation. Do not leave the rejected small button and side-by-side link.

## Final review gates

Fresh images required at 1586 × 992 and practical1440 × 900 /1920 × 1080, including
config authentication, edit form and retention edit. Inspect exact geometry, visual
weight and control formatting, not merely whether cards exist. Verify shorter-height
scrolling/footer reachability and no horizontal collisions. Record concrete remaining
deviations; acknowledge data-driven differences separately. No prior acceptance carries
forward, and no direct interaction/connectivity proof follows merely from screenshots.
