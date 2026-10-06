# Private Runner relationship cards and independent UI acceptance

Observed: 2026-10-06
Confidence: verified

## Sources and Method

The user approved the generated `view-v2.png` under
`output/imagegen/2026-10-06-private-runners-relationships/` and requested an
independent UI reviewer. The [desktop contract](../contracts/desktop-experience.md#private-runner-relationship-cards--2026-10-06)
and [catalogue contract](../contracts/mcp-catalogue-and-profiles.md#brain-connections-and-profiles)
govern the slice. Inspect `RunnerConnections.tsx`, `McpPrivateRunners.tsx`, the
scoped CSS, typed protocol summary and server projection. Canonical HTTP reads,
CUA browser interaction, tab-only Fetch fixtures and actual rendered captures
establish the evidence below; no tool execution was needed for this UI change.

## Observations

Private Runners now shows a compact count/Add toolbar, paired device under the
runner title, separate connection status/Enabled/Edit, assigned MCP scope and
approved tool chips, linked tool groups, and a slim device/setup footer. Existing
Edit retains the relationship body. Add/Edit still uses the original device,
revision, idempotency, pending, retry and archive guards.

On SWEG — test, the exact Ubuntu runner binds Ubuntu filesystem demo to its
paired device, with Brain-wide scope, three approved read tools and the Ubuntu
demo files group. Both owner and demo reader canonical APIs return this summary
binding. Reader catalogue definitions are withheld; full connection and definition
reads remain 403. Reader tools use existing authorized cached discovery, not an
administrator endpoint. Actual reader UI, through a temporary read-only local
proxy, shows these three tools with no Add/Edit/setup actions and a device UUID
fallback instead of another account's device inventory.

The summary exposes only a nullable canonical private runner UUID. Central/local,
malformed, simple and uppercase UUID references are excluded. Legacy summaries
without the new field remain deserializable. Metadata queries share one definition
read or one bounded profile traversal, then filter per connection. Two fixture
connections in one profile used exactly offsets 0 and 20, producing separate
21-tool and one-tool counts. Exact group environment was passed unchanged.

Thirteen focused browser checks passed: labelled tool loading; denied metadata;
shared complete pagination; old-server missing binding; no-Use gating; pending
fields/links; failed-save draft retention; original-revision/idempotent retry;
failed-refresh hiding; exact environment; qualified reader-empty state; and both
missing-resource link notices. Save responses were intercepted entirely in the
owned tab. Actual registration name, device, Enabled and revision stayed unchanged.
Real links open the exact MCP Tools inspector and tool-group card. Real inline
Add/Cancel and changed-name Edit/Cancel preserve the registration.

At 1384×1473 the heading/card share x280 and the card is 1072px wide; at
2260×1314 they share x414 with a 1200px cap. Read height is about 307px at both
sizes; neither document has horizontal overflow. The independent reviewer passed
actual read/Edit wide and laptop plus Add laptop against the approved mockup.
The reviewer found stricter binding validation, duplicated profile reads and a
missing-group notice during source review; all were corrected and re-reviewed.

Artifacts in `output/private-runner-relationships-2026-10-06/` include
`read-wide.png`, `edit-wide.png`, `read-laptop.png`, `edit-laptop.png`,
`add-laptop.png`, `reader-laptop.png`, `geometry.json`, `api-proof.json` and
`browser-checks.json`. Final browser error/warning logs were empty. Overrides,
intercepts, owned tabs and the temporary proxy were removed; original user tab
and demo resources remain.

## Translation and Limits

These are routing relationships and approved metadata, not a grant or proof of
installed/reachable tools. Connected describes the runner lease. Device inventory
is current-account only. Existing filtered catalogue visibility and Use/Manage/Share
admission remain authoritative. The independent review used current source and
saved rendered captures because the child could not open IAB; functional browser
interaction was performed by the parent.

The local stack uses image
`sha256:bf43f20cd956f3eb1e1af7333ec995a9a5e2294bf8956e80c81c945fdf2e017e`.
A cached Linux dev-profile server build and production web build were layered
over the verified running image, then started through `./scripts/stack.sh up`.
Readiness returned true and the existing Ubuntu demo remained running/Connected.
Two focused Rust tests, web build/type/design checks, governance32, whitespace
and CodeGraph passed. No migration, credentials, grants or execution changes.

## Follow-up

This implementation supersedes the preceding spacing slice's proposal-only
relationship state. No remaining blocker for this slice. Existing native-host
acceptance and unrelated active packs remain separate. Version N/A;
uncommitted, no push or external release.
