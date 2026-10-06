# Brain chooser, icons and personal Agents — 2026-10-03

## Authority and scope

The user removed activity from the global Brains chooser, requested uploaded Brain
artwork, selected the grouped list Agents proposal, and clarified that global
Agents is personal even for an installation owner. Cross-user agent attribution
belongs inside the accessible Brain. The redundant Devices page is consolidated
into Agents Manage access; native pairing and own-account revocation remain.

Authority: [desktop contract](../contracts/desktop-experience.md),
[pairing contract](../contracts/platform-device-pairing.md),
[visual epic](../roadmap/epics/desktop-visual-experience.md), and
[execution pack](../roadmap/execution/archive/desktop-brain-identity-and-navigation.md).
This supersedes the global chooser activity and global-owner roster scope in the
[preceding control-panel evidence](desktop-live-control-panel-2026-10-03.md).
TV is separate and unchanged.

## Implementation evidence

- `BrainsPage.tsx` removes the pipeline import/selection state. Main card links
  open the Brain; existing search/access/archive filters and IDs remain.
- `BrainForm.tsx` and `BrainIcon.tsx` support PNG/SVG/ICO up to 512 KiB, static
  image-context conversion and preview, replacement/removal, generic fallback,
  and retry after partial metadata success without duplicate creation. Opening
  the form reloads current metadata, preserving a renamed Brain on icon-only save.
- Migration 032 owns canonical PNG bytes and revision on the Brain row. The
  server accepts only PNG, enforces a 1024px decoder limit and 8 MiB allocation
  budget, re-encodes at most 256px and strips original metadata. GET is authorized,
  `image/png`, `no-store`, `nosniff`. Browser/admin/CSRF checks protect changes.
  No image bytes or original uploads enter audit/command receipts or a file store.
- `/api/agents` is own-account-only for all users. The frontend groups actual
  observed uses by Brain, omits empty groups, searches names/hosts/Brains and
  opens details on demand. No always-visible observations column or headline
  explanation block remains.
- Migration 033 exposes only device identity/last-use within an authorized Brain.
  Foreign credential timestamps are null and `can_revoke` is false. The latest
  still-retained capture supplies a Brain-local legacy host fallback. Effective
  capture expiry is enforced before cleanup. Private MCP call/payload RLS is
  unchanged; a reader can see contributor metadata without reading private calls.
- Agents Manage access owns complete own-account credentials and history, including
  unused/pending connections. `/devices` redirects there, preserving eight-hex
  codes, sign-in return, explicit approval/decline and account-specific revocation.
  The drawer remains usable when the Agents roster request fails.

## Dependencies and design evidence

No new animation or 3D dependency. Existing Motion handles finite list/layout
transitions with reduced-motion support; Activity keeps its truthful pipeline.
Added `image` 0.25.10 with default features disabled and PNG only.
Context7 `/image-rs/image` was queried for decoder limits and PNG encoding; its
main-branch `ImageReaderOptions` examples differ from 0.25, so the installed
`ImageReader` API was verified by compilation and decoder tests. Primary browser
reference: [MDN SVG as an image](https://developer.mozilla.org/en-US/docs/Web/SVG/Guides/SVG_as_an_image),
which documents disabled scripting and external resources in image context.
No SVG is inserted inline, loaded in an iframe/object, or served by the backend.

The built-in image-generation tool produced a flat cream/sage comparison using
labeled illustrative data. User selected A (grouped list), then clarified personal
scope. Prompt: `.cache/agents-by-brain-proposal-prompt.txt`; image:
`.cache/agents-by-brain-proposal-20261003.png`. Generated names/timestamps are
proposal content only and are not product data.

Independent read-only reviewer identified migration registration, stale editor
state, quoted SVG references, effective capture expiry, local host fallback and
contradictory contract statements. These were corrected; no material source issue
remained on recheck. Validation results below are independent of that review.

## Validation

- `.cache/brain-identity-api-final.log`: real HTTP/database icon authority,
  normalization/transparency, malformed/oversized rejection, replacement/removal,
  access loss, whole-Brain deletion and retained-journal replay against a simulated
  restored Brain row passed. This is SQL replay proof, not a physical backup restore.
- `.cache/brain-identity-unit.log`: two decoder tests passed, including excessive
  dimensions, wrong format, malformed input and canonical stability.
- `.cache/brain-identity-agents-api.log`: two real database tests passed, including
  owner-global isolation, shared-Brain contributors, hidden foreign dates,
  own-only revocation, unchanged private MCP visibility, Brain denial, effective
  capture expiry and legacy host fallback.
- `.cache/brain-identity-ui.log`: eight grouping/search/pairing/history/pipeline/
  accessibility/filter tests passed. The first icon test failed on test selectors
  (required-name label, then a hidden sidebar option); those selectors were fixed.
- `.cache/brain-identity-icon-ui-final.log`: complete icon browser workflow passed:
  real SVG (including quoted local gradient), PNG and ICO conversion/upload,
  transparent preview, upload failure/retry without duplicate creation, rename/
  reopen preservation, invalid/oversized rejection and removal. All nine focused
  browser cases therefore passed; first-run test-selector failures remain recorded.
- `.cache/brain-identity-live/proof.json`: local deployed browser at 1920px,
  2026-10-03 13:16 UTC, readiness true and no page errors. Sixteen actual Brains;
  twelve own credentials; one visible Brain group with three used agents;
  thirty actual pipeline inputs on that Brain. The global chooser has no pipeline.
  Devices redirects into the access drawer. Icon conversion preview was cancelled
  without creating a Brain or persisting demonstration artwork. Light palette
  remains `#f7f4ec` even with the browser configured for a dark system theme.
  Actual captures: `brains.png`, `agents.png`, `manage-access.png`,
  `brain-activity.png`, `brain-agents.png`, `icon-preview.png` in that directory.
  The earlier grouped fixture screenshot caught a layout transition; the stable
  deployed `agents.png` is the presentation evidence.
- `.cache/brain-identity-clippy.log`: workspace/all-target clippy with warnings
  denied passed. Frontend build, typecheck/design and 176-operation schema
  generation passed. Existing large main-bundle warning remains.
- CodeGraph sync passed; `validate.sh` passed all 32 governance tests.

## Delivery boundary

Version: N/A. Commit: uncommitted. No push or release. The existing local Compose stack was rebuilt and migrated through 033; API and
worker restarted successfully. `.cache/brain-identity-deploy.log`, Docker build
log and live proof record this boundary. Before/after inventory is identical:
16 Brains, 1 account, 52 source versions, 117 claim revisions and 34 devices.
No external deployment. The independent reviewer cleared source and focused
acceptance evidence; the primary agent separately verified deployment.
