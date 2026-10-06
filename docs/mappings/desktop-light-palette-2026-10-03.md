# Desktop light palette restoration

Observed: 2026-10-03
Confidence: verified

## Sources and Method

The user explicitly requested the original light colour theme with the new
visual features retained. Compared the working tree against `HEAD` for the
original Atlas primitives, semantic roles, button styling and SVG colours.
The [desktop contract](../contracts/desktop-experience.md) and
[design system](../roadmap/desktop-experience/design-system.md) now record that
correction. This is the owning epic's isolated presentation fix.

## Observations

- Restored cream canvas `#f7f4ec`, paper surfaces `#fffdf8`, sidebar `#eee9de`,
  ink `#101b2a`, sage accent `#3d6b59`, original button colours, SVG logo and
  browser theme colour. Motion tokens, category badges, pipeline behaviour and
  Rust CLI presentation remain present.
- Category foregrounds use darker shades of the original accents. Calculated
  contrast on tinted canvas, card and sidebar surfaces is 4.65–5.06:1 at the
  worst combination for each category. This is not full accessibility proof.
- Terminal cards use paper and cream; safe ANSI white/bright-white resolve to
  readable ink. Fixed terminal title-bar and error-chip CSS variable names to
  match the actual semantic tokens. A source-component rendering fixture proved
  the title bar, body and ANSI colours. Live tool details correctly reported
  expired retained output, so no fresh tool execution was used for this fix.
- Rebuilt the existing local Compose stack with `./scripts/stack.sh up --build`.
  `/health/ready` returned `{"ready":true}`. Owner browser checks confirmed the
  light palette even with the browser preferring dark mode, real pipeline reads
  at 1440×900 and 1920×1080, and a 1.2s status pulse reduced to 0s under reduced
  motion. No browser page errors were observed. No model or tool call was made.
- Frontend build/typecheck/design checks, `./scripts/validate.sh` (32 tests),
  CodeGraph sync and `git diff --check` passed. Vite retains a non-failing large
  bundle warning. Proof scripts, computed values and screenshots are in ignored
  `.cache/light-theme-proof/`; build/validation/deployment logs are in `.cache/`.

## Translation and Limits

Only the palette fix is closed here. The prior three visual packs remain
historical deliveries. The TV route, component, styles and Activity toggle were
already present at the start of this task, contrary to the supplied handoff's
“implementation not started” statement. Its active pack has no completed
acceptance evidence or closeout. This task only adapted its inherited palette
and corrected its future light-theme requirements.

Source inspection also found TV still projects cached claims after a failed
refresh and lacks the Memory surface's expiry gate. Those are unfinished TV
acceptance work; they were not changed or tested by this colour correction.
No new external/dependency interface was introduced, so no new dependency lookup
was needed. Version: N/A; commit: uncommitted. No commit, push or release.

## Follow-up

Finish TV invalidation/expiry behaviour and its real-data, empty/error/access,
cycling, exit and reduced-motion acceptance. Then archive its pack and close the
visual epic, indexes, evidence and handoff. The nine research adoption ideas
remain deferred; this correction does not start that backlog.
