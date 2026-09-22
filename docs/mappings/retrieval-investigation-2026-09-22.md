# Desktop investigation interface and implementation evidence

Observed: 2026-09-22
Confidence: verified local implementation and retained normal runtime.

## Interface evidence

Context7 resolved `/tanstack/query` and returned disabled-query, cancellation and
stale-data-on-error guidance. Some examples refer to the developing main branch
and a newer `query()` API; these are not an upgrade instruction. The installed
TanStack Query 5.102.0 types and the official
[disabled-query guide](https://tanstack.com/query/v5/docs/framework/react/guides/disabling-queries)
and [cancellation guide](https://tanstack.com/query/v5/docs/framework/react/guides/query-cancellation)
support retaining disabled paid recall with explicit `fetchQuery` and consuming
AbortSignal. A query may retain data after a refetch error, so render eligibility
must check failure as well as data presence. No dependency upgrade is required.

CodeGraph was synchronized before structural navigation. Existing `ClaimDialog`
already supplies history, review and procedure/handover inspection; `EvidenceDialog`
resolves exact evidence; `GraphExplorer` supplies the qualified desktop renderer.
Reuse those owners. Initial findings: recall detail defaulted to latest knowledge
rather than the response cutoff; epoch polling only covered graph-enabled recall;
detail errors could leave cached content visible. Canonical item/evidence deadlines
already exist server-side and can be exposed without new retention policy.

The [accepted contract](../contracts/retrieval-investigation-ui.md) governs this
work. The user-requested reference agent completed a read-only Atlas/Cognee audit.
Evidence-before-belief, explicit trust, bi-temporal validity, editing memory and
resolving conflicts support exact version inspection, independent state dimensions,
the frozen cutoff and links to existing optional review. They add neither a human
approval queue nor a different eligibility policy.

Cognee's `modules/business/computeSourceDetail.ts` derives source summaries from
loaded data; its `panels/NodePanel.tsx` joins selection, related evidence and path
controls. Recollect similarly rearranges returned provenance and reuses canonical
inspectors. Cognee's `app/(app)/search/SearchPage.tsx` flattens results into text,
and its source cards list names instead of supplying exact retained-version
inspection. Its client path helper uses undirected BFS; Recollect retains native
directed paths. Neither separate reference checkout was modified.

The review also identified nested review/conflict dialogs rendering saved content
before their parent's error branch, caller-dependent correction invalidation,
and an explorer that would request a broad overview before receiving a recall
center. The delivered implementation clears the complete investigation on observed
canonical/access changes, hides stale detail payload, shares mutation invalidation,
and seeds the exact graph center before its first request.

## Acceptance

Implemented `RecallResults.tsx` (three views, source attribution and time/scope/
trust), `RecallGraphDialog.tsx` (same-selection native exploration/path), explicit
result limits and shared deadline/error handling. Claim links now use the server
response's frozen cutoff. Graph settings and optional path controls collapse on
the investigation dialog so the actual graph remains visible on desktop. Existing
standalone graph controls keep their behavior. All normal review/mutation policy
remains owned by memory, with no new provider operation or schema migration.

Visual review found oversized overlapping labels on a two-node path. Cytoscape's
installed 3.34.3 layout options and Context7 `/cytoscape/cytoscape.js` confirm
`nodeDimensionsIncludeLabels`; layouts now include labels in spacing and automatic
fit caps enlargement while retaining manual zoom. The existing small-path and
500-node/2,000-edge browser flows passed again after this adjustment (four cases,
35.8 seconds, dense rendering and keyboard selection 1,155 ms). Final sparse-path,
disagreement and SWEG screenshots were inspected directly.

Executed on 2026-09-22:

- **Seven browser scenarios passed in 53.2 seconds**: three new investigation
  flows plus existing recall, graph recall, graph explorer and review regressions.
  Actual PostgreSQL/Neo4j calls prove a disputed pair with an accepted positive
  control, strict exclusion, exact old source inspection after a later source
  version, missing-reference evidence, optional review, procedure/handover details,
  native same-selection graph exploration and a one-edge path from recalled claim
  to evidence. Historical review cannot mutate until latest is explicitly selected.
- A real late claim revision appears only after an explicit switch to latest;
  the initial detail request equals the recall response's frozen cutoff. Holding
  only status observation models the interval before the next remote-change poll.
  Removing that hold clears text-only recall without a query resubmission.
- A one-result query still exposes a canonical omitted-conflict link. Changing
  views issues no claim/retrieval fan-out; explicit related inspection does not
  change copied context. Clipboard denial is visible. A held recall response
  released after a filter change cannot repopulate the new selection; a fresh
  positive query returns the correct independent claim.
- Detail refetch failure hides old bytes, and a subsequent explicit refresh
  succeeds. Short advertised deadlines exercise independent evidence and complete
  text-context clearing. A controlled permission failure clears all inspection
  state. Actual source Erase while nested in review/conflict evidence removes the
  whole investigation and dialog; unrelated retained evidence remains searchable.
  Source markup remains inert and all these fixtures record **zero model calls**.
- **Eleven affected platform retention scenarios passed in 46.43 seconds**,
  including the new live-evidence display deadline, required handover expiry,
  graph hidden-input and publication gates, retained excerpt independence,
  erasure closure and older-database replay. The new API fixture verifies that
  an omitted candidate cannot shorten independent returned context and that
  an expired source does not make a separately retained qualified claim expire.
- **13 workspace tests passed**; Clippy workspace/all-targets with denied
  warnings passed. OpenAPI remains **125 unique operations** with additive
  nullable deadline fields; generated types, browser build/typecheck and formatting
  passed. Governance validation and **32 checker tests** passed.

Initial test failures were corrected without weakening product checks: erasure
now intentionally unmounts its parent investigation instead of retaining an
outcome dialog with stale context; reference-only coverage correctly gives
insufficient support instead of a complete no-match; Mantine's required-label
marker needed a prefix selector. Final tests exercise these resulting states.

Ignored proof: `.cache/investigation-ui-complete.log`, `investigation-retention-final.log`,
`investigation-workspace.log`, `investigation-final-clippy.log`, `investigation-api.log`,
`investigation-governance.log`, and desktop disagreement/graph screenshots. Browser
deadline/error controls prove UI behavior; canonical API eligibility is separately
proven by the real database tests. No actual-model quality claim is added by this
presentation slice. The existing main-bundle size warning remains non-blocking.

## Normal runtime preservation

The normal API/worker and updated desktop bundle run at `http://127.0.0.1:8787`;
`/health/ready` returned `ready:true`. A real exact-only SWEG fact lookup returned
one attributed item, 1,630 context bytes in 60 ms, without a semantic or graph
retrieval channel. The browser inspected the exact retained `variables_os.tf`
source at the selected commit, then opened native graph exploration directly
around that fact: **74 nodes / 151 directed relationships**, with the same
repository, environment, exact manifest and snapshot. No extra broad overview
request was needed and no browser exception occurred.

The before/after PostgreSQL inventory matched byte-for-byte: the same **seven
Brain identities, 33 model requests, zero queued/running jobs and 20 migrations**.
No new model request or customer filesystem read was needed. The script's first
selector attempt ran before catalogue loading; waiting for that real response
and searching the repository dropdown resolved the proof timing without a product
change. Both separate reference checkouts remain untouched.

Ignored proof: `.cache/investigation-runtime.json`, `investigation-runtime.log`,
`investigation-preservation-before.json`, `investigation-preservation-after.json`,
`investigation-sweg-recall.png`, `investigation-sweg-graph.png` and
`investigation-ui-visual-final.log`. Local delivery is uncommitted, not deployed
to the historical transfer VM or an external service.
