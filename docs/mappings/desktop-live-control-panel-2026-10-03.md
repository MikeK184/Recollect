# Flat desktop control panel and observed processing

Observed: 2026-10-03
Confidence: verified

## Sources and method

The user approved the flat light Flow deck and explicitly required real current
or last-observed activity, including concurrent agents and the information each
contributed. The [desktop contract](../contracts/desktop-experience.md#live-control-panel-amendment--2026-10-03),
[visual epic](../roadmap/epics/desktop-visual-experience.md) and owning execution
pack govern this implementation. The ignored comparison/prototype artifacts in
`.cache/visual-proposal-20261003/` are illustrative design evidence, never data
inputs for the product.

Used the existing schema and producer/worker implementations to verify the exact
chain: capture binding actor/device → published capture → source version →
`source.process` job → learning run and its exact job → recorded dispositions.
Source metadata identifies the contributor; a policy-authored learning run does
not turn the policy author into the source contributor. Graph generations lack a
per-source causal relationship and therefore appear separately.

Dependency guidance was retrieved with Context7 from `/websites/motion_dev`
(layout, accessibility and reduced motion) and `/xyflow/xyflow` (custom nodes,
edges, read-only controls and performance). Primary interfaces:
[Motion layout](https://motion.dev/docs/react-layout-animations),
[Motion accessibility](https://motion.dev/docs/react-accessibility),
[React Flow edges](https://reactflow.dev/examples/edges/animating-edges),
[React Flow performance](https://reactflow.dev/learn/advanced-use/performance).
Installed Motion 14.0.0 and React Flow 12.12.0. Existing Cytoscape remains the
knowledge graph renderer. No decorative 3D runtime was added.

## Implemented behavior

- Cream/ink/sage global Brains, Agents and Team surfaces; compact selectable Brain
  cards with a separate Open Brain action; create/input/agent/team/runtime drawers.
  Existing sign-in, pairing, grant, invitation, reset, disable and history paths
  remain available. Team has Accounts, Invitations and Account activity tabs.
- `/api/brains/{brain}/pipeline` is an authorized read projection, capped at 30
  inputs with an explicit overflow marker. Active inputs take priority over recent
  terminal activity. Receipt timestamps order delayed uploads correctly; retry
  priority and last-finished learning ordering retain the relevant exact job/run.
- The selected Brain refreshes every two seconds while visible. First history and
  unchanged reads never replay. Newly observed transitions pulse only their own
  edge; current worker activity has a status pulse. Multiple contributor/device/
  binding/session/agent contexts retain separate identity. Inspectors link to the
  exact retained source version and job, and label memory links as current memory.
- Successful learning may produce no memories. Recorded accepted/proposed/reused/
  revised/retired/blocked/conflicting dispositions can overlap and are not summed
  into invented new-memory totals. Independent Brain graph status has no causal
  edge from an individual source.
- Failed reads clear payload and inspectors. A server deadline (at most six seconds
  and no later than source expiry) gates display; the client subtracts request
  latency, clears at the deadline, pauses hidden views and revalidates on return.
  Selection switches use separate keys and abort signals. Removed inspected inputs
  cannot silently substitute another input during the drawer transition.
- Agent credential validity is explicitly separate from presence. Global owners
  retain the existing installation-wide device metadata scope; other users see
  their own. Brain usage remains limited to accessible Brains. Diagnostics checks
  only the actual core stores; latency bars are cumulative request counts.

## Verification

- The focused disposable API test exercises two real device capture admissions,
  distinct provenance, a delayed upload, source-free lifecycle input, manual
  import, exact jobs, a real worker observed in-flight through a synthetic provider,
  zero-output success, a retried older job, out-of-order learning finishes, bounded
  active-first results, denied/revoked access, expired and erased input exclusion.
  It passed; no live provider spending was required. Failed fixture setup runs
  were corrected and their precisely identified disposable databases/artifacts
  cleaned after checking for external graph/privacy work.
- Browser checks exercise actual source imports and exact source links, initial/
  unchanged history, edge-specific changes, pinned inspection, removed inputs,
  failed and stalled polls, and reduced motion. Existing Team invitation, shared
  access revocation, account disable, diagnostics failure clearing, and list-history
  tests passed. Final 4/4 control-panel checks include populated agent rows/inspector and all four filters plus all diagram nodes after resize at 1024/1440/1920. The separate regression run passed 7/7; Team passed with its provider-dependent OIDC case explicitly skipped.
- Rust clippy for server/protocol and all their targets passed with warnings denied.
  Frontend typecheck, design checks and production build passed. Governance
  validation passed 32 tests. Dependency audit has zero vulnerabilities after a
  narrow transitive brace-expansion update. Existing main-bundle size warning
  remains; the shared pipeline chunk is about 194 kB / 63 kB gzip.
- Independent read-only review corrected retry/run ordering, timestamp visibility
  races, inspector substitution, status priority and desktop clipping. Source
  review and executed test evidence remain distinct.

Logs and captures are ignored under `.cache/control-panel-*`,
`.cache/pipeline-api-test-final.log`, and `.cache/ui/control-panel-*.png`.

## Runtime closeout

Built `recollect-dev:local` and ran `./scripts/stack.sh up`; API, worker,
PostgreSQL and Neo4j became healthy. The deployed owner browser read 30 actual
pipeline inputs spanning four origin contexts, with the real source/run outcomes
shown as last activity. Populated agent roster/inspector, Team, runtime and create
surfaces passed; screenshots waited for drawer transforms to settle. The original
cream canvas remained `#f7f4ec` even under dark browser preference. No page errors.
Live evidence is in `.cache/control-panel-live/`, including `proof.json`.

Before/after inventory is identical: 16 Brains, 1 account, 52 source versions,
117 claim revisions and 34 devices. The live check added no content, accounts or
agent credentials. Processing transitions were tested in disposable fixtures;
the deployed read check does not pretend historical events are currently running.
Version: N/A; commit: uncommitted. No push or release.

## Remaining scope

The separately active TV slice still needs its cache/expiry/failure handling and
acceptance. It is not completed by this change. Overall visual epic closeout
follows TV acceptance. The nine claude-mem research ideas remain deferred.
