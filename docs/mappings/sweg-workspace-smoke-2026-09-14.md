# SWEG workspace smoke test

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The user authorized `/Users/mike/devops/customer/SWEG` as real test input for
Recollect's existing features. The [workspace contract](../contracts/evidence-workspace-scope.md)
and [collections contract](../contracts/evidence-collections.md) govern those
features. No customer service, Terraform operation or remote Git operation was
needed. Git and native-store interfaces remain those in the
[workspace proof](workspace-scope-proof-2026-09-14.md).

Enumerated existing Git markers without following directory symlinks, excluding
hidden/build/dependency directories and nested selectors. The scan visited 614
directories and 2,469 entries without reaching its bounds. SWEG had no Recollect
selector. An opt-in [native probe](../../crates/agent/examples/workspace_probe.rs)
therefore passed the explicit checkout list to the same bounded Git observation
helper used by the companion, then used a dedicated paired device to call the
existing refresh API. This is deliberately recorded as a partial explicit-list
observation, not selector-based native discovery.

Created the persistent local Brain **SWEG — test**, UUID
`5c054930-d266-4c18-a42b-942729f942aa`, through the browser. Paired an isolated
native profile, exercised the API/CLI/browser, then revoked the probe device and
removed its own OS-store credential. The Brain, observed catalogue and working
history remain available for the user at the local UI.

## Observations

| Behavior | Result |
| --- | --- |
| Local inventory | 80 checkouts, all with available metadata; 79 distinct normalized origins; one dirty checkout and zero unknown dirty states. These are local observations, not remote availability or deployment proof. |
| Stable identities | Repeating the authenticated refresh retained all 79 repository UUIDs. The two checkouts sharing an origin remained separate observations. |
| Task scope | Native start, fork, change, bind, inspect and close succeeded. A prior operation retained its original repository; the child retained its independent scope after its parent changed. The child was closed and the parent remains available for UI exploration. |
| Collections | Created Test observations, Repository inventory and Local test views. Imported and processed one generated metadata report. Its content is test output, not customer source text. |
| Browser | Real isolated Chrome signed in, created the Brain, approved pairing, browsed repositories/checkouts and inspected task history and the processed report. No page errors; the 390-pixel mobile layout had no horizontal overflow. |
| Preservation | Compared inode, size, modification time and change time for 480 Git metadata paths before and after the probe: zero changes. No customer selector was created. No source files, credentials or raw remote URLs were imported. |

Local proof artifacts and the resumable browser driver are in ignored
`.cache/sweg-proof-64a650a9726e423ea3319a7854c604d0/`. Screenshots include
`repositories.png`, `checkouts.png`, `task-history.png`, `source-report.png` and
`mobile.png`. Inputs and refresh JSON contain only checkout paths and normalized
Git metadata. The helper visibility change has no new wire/schema/selection
behavior; its small-fix exception is recorded in the owning evidence epic.

## Validation and Limits

The real service, native profile and browser scenario passed. The existing
native discovery fixture passed again, and agent clippy passed with warnings
denied. Governance validation passed with all 32 checker tests. Version: N/A;
changes remain uncommitted. The test used the normal local Recollect installation,
not an externally deployed service.

This does not prove exact committed extraction, search, graph relationships,
customer runtime state or automatic session capture. Those product slices remain
open. Selector-based discovery still has its separate temporary-fixture proof;
the customer directory was not configured as a managed Recollect workspace.
Future SWEG publication tests must preserve that write boundary and distinguish
committed source evidence from locally dirty and deployed state.
