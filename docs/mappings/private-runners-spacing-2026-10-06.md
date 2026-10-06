# Private Runners spacing and relationship-display proposal

Observed: 2026-10-06
Confidence: verified

## Sources and Method

The user requested a spacing correction first, plus an explanation of how to
make each runner's device, connections, tools and tool access clear. The
[desktop contract](../contracts/desktop-experience.md#private-runner-cards-and-inline-editing--2026-10-06)
governs this small presentation fix. Source inspection covered scoped CSS,
`McpPrivateRunners.tsx`, generated DTOs and `crates/server/src/mcp.rs`.
CUA used a temporary background tab on the actual SWEG installation. Read-only
canonical API calls checked the existing Ubuntu demo relationships; no grants,
registrations, tools or connection settings were changed.

## Observations

The old route rule centered the entire runner page at x654 on a 2260px viewport,
while Connections and Tool access started at x414. Removing its automatic inline
margins restores the common x414 edge for heading, tabs and cards, with the
existing 1200px cap retained. Removing the toolbar's top margin produces a 16px
tabs-to-toolbar gap and retains the 20px toolbar-to-card gap.

At 1384×1473, heading/tabs/cards share x280 and 1072px width. At 2260×1314 they
share x414 and 1200px width. Read cards remain 135px high and Edit cards 142px;
neither view has horizontal document overflow. Actual Edit/Cancel and Add/Cancel
keep the same alignment and preserve the immutable paired-device badge. No Save
was issued for this CSS-only correction. Browser error/warning logs were empty.

Artifacts under `output/private-runners-spacing-2026-10-06/` are
`after-wide.png`, `edit-wide.png`, `after-laptop.png`, `edit-laptop.png`,
`geometry.json` and `relations.json`. Explicit CDP screenshot clip plus
`captureBeyondViewport:true` preserves the full logical viewport. Earlier tiled
captures were replaced; the invalid baseline was removed.

Canonical reads confirm the connected Ubuntu filesystem demo registration binds
to the paired device Ubuntu filesystem demo runner. Its exact `private:UUID`
connection is Ubuntu filesystem demo, Brain-wide, with three approved tools;
the Ubuntu demo files group contains that connection.

## Translation and Limits

Recommended future card: paired device/status in the header, then compact
Assigned connections rows with scope, Approved tools count/names and linked
Tool groups chips. These are routing and approved metadata; caller Use and
environment checks still decide admission. Do not infer OS/location or currently
installed tools from names, registration or the runner lease.

Connection summaries omit `runner_reference`; full connection detail is
admin-only. An administrator can derive the exact relationship with existing
reads. A reader-safe summary needs separately authorized product work; denied
detail must never become an empty No assigned connections state. This proposed
relationship section has not been implemented.

The production web build includes design/type checks. Required governance lint
and 32 tests, whitespace and CodeGraph sync passed. The initial governance check
correctly rejected an in-progress slice under a completed epic; the temporary
active state and final complete closeout were reconciled. An independent UI
review checked the corrected full laptop/wide read/Edit artifacts.

Normal stack startup deployed frontend-only image
`sha256:c65591fe3ce16723f66037acc433d1d4efe295e56baa6ba10100fd983463fcab`.
The prior base image was unavailable locally, so the replacement layer used the
verified current running image, retaining its backend. Readiness returned true;
the existing Ubuntu demo container stayed running. No runtime schema change.
Browser override cleared and owned tab closed; the original user tab preserved.

## Follow-up

Present the relationship proposal for user selection before implementing it.
Version N/A; uncommitted. No commit, push or external release.
