# Desktop memory investigation

Status: accepted

## Source

The full product goal authorizes routine decisions and implementation. This
contract completes the [hybrid retrieval epic](../roadmap/epics/hybrid-retrieval.md)
using [canonical recall](retrieval-exact-and-lexical.md),
[graph fusion](retrieval-graph-fusion.md), [claim history](memory-claims-and-time.md),
[review](memory-review-and-corrections.md), [memory forms](memory-procedures-and-handovers.md)
and [graph exploration](graph-exploration.md). Mobile is deferred by the user's
2026-09-22 instruction. Normal memory maintenance remains autonomous; investigation
and human review are optional controls.

The 2026-09-26 [desktop contract](desktop-experience.md) moves this unchanged
evidence investigation into Search evidence under the contextual Ask route.
[Evidence-backed answers](retrieval-answers.md) is the separately authorized
generated-answer consumer; it does not replace these bounded search semantics.

## Contract

### One bounded investigation

Extend the existing Brain recall panel with Matches, Disagreements and Sources
views over the same returned context. These are presentations of one response,
not additional retrieval lanes, a generated answer or an alternative eligibility
policy. Switching views creates no model request and does not change copying,
ranking, context budget or source-diversity selection. Filters and explicit
submission retain all existing exact/text/semantic/graph contracts.
Expose the existing 1–20 result limit (default ten) beside the context budget.
Page displayed disagreement links ten at a time without fetching omitted claims.

Keep a visible summary of the submitted Brain, named repository/area/environment
selection, collection, exact manifest, mode, knowledge cutoff and optional fact
time. Display the response's frozen server knowledge time even when the user
selected latest knowledge. Empty scope dimensions are labeled as all in this
Brain, not as deployment or execution authority. Missing catalogue names fall
back to exact IDs; never silently substitute another scope.

Matches expose separate review, freshness, operational, lifecycle and acceptance
origin, relevant fact/knowledge time, canonical qualifications, source revision/
location and recorded discovery channels. A policy acceptance is not human review.
Semantic similarity, graph proximity and source coverage do not prove an answer.
Strict modes and historical selections remain explicit. Procedures and handovers
open the existing forms with conditions, outcomes and contributor provenance;
no execution action is added.

Disagreements use canonical conflicting-claim IDs only. Show conflicting returned
claims together with their states and applicability; no value parsing, newest-wins
choice or model inference creates/resolves a conflict. Related claims omitted by
the bounded retrieval response are labeled as outside this result set. An explicit
inspection may follow that exact same-Brain claim through its existing owner
handler, showing its actual applicability; it does not add it to copied context
or claim it matched the query. No background fan-out fetches omitted claims.
No disagreements in this result set is not proof of global consistency.

Sources deduplicate returned provenance by exact evidence kind/ID, retain each
used location/span, show availability/repository/commit and identify the matched
records it supports. This is evidence attribution, not an independent-source
counter. Missing evidence remains visible and inspectable as an unavailable
record; no generated text or stale search chunk substitutes for its bytes.

### Canonical inspection and graph navigation

Claim/history links start at the response's frozen knowledge time and fact-time
selection. The existing owner dialog permits an explicit switch to latest or
another retained revision, labeling historical knowledge. It retains real review,
correction, conflict resolution and Erase authority; reader/archived access and
stale revisions cannot gain mutation rights through investigation. Evidence links
resolve exact retained versions, not the latest source by title. Closing detail
returns to the same investigation view while it remains valid.

Each graph-compatible current result can open the existing desktop explorer in
a bounded dialog. Preserve Brain, collection, parent repository/area/environment
selection, exact manifest, fact time and eligibility mode. A graph-fused result
uses its returned graph selection; a plain repository fact uses its exact returned
snapshot, and a plain claim/source uses the knowledge graph. Do not follow a
newer repository snapshot, widen the parent selection or guess an environment
manifest. Manifest declarations have no direct center and explain that limit.

Use the result's canonical kind/revision key as center. The graph owner rechecks
current qualification and supplies native reachability, selected evidence and
optional native shortest paths. Graph calls are explicit read-only inspection
and create no model request. The dialog labels this as a current graph read,
not reconstruction of the recall cutoff. If the graph's memory epoch differs
from the recall response, clear the investigation and require an explicit new
query. A missing/withheld center or projection remains an actionable error;
never silently fall back to a broad overview or another scope.

Historical mode or explicit historical knowledge selection cannot open a current
graph as if it represented that past. Explain the limitation and retain exact
history/source inspection. Native graph computation, display limits, expiry and
path semantics remain owned by the existing graph contract.

### Invalidation, privacy and requests

Add nullable `expires_at` to recall responses: the earliest retention deadline
of returned items and their required live dependencies, plus the complete graph
input deadline when used. Derive it from already qualified final packed items;
omitted/expired items do not shorten a non-graph result. Null means no known
clock deadline, not immutable authority. Add the existing canonical row deadline
to exact evidence-detail responses so a source opened from a retained claim can
clear independently when its raw content expires. No schema migration is needed.

All recall presentations and derived inspection state clear together on a filter,
Brain, canonical cache invalidation, observed access/epoch change or advertised
expiry. Reuse the existing cheap graph-status metadata read to monitor the Brain
memory epoch for every displayed recall, including text-only recall. This endpoint
uses PostgreSQL and does not require an available physical Neo4j projection.
Poll at three seconds while context is shown; a failed authorization/status read
clears it. A remote mutation may remain visible until observed, so this is bounded
polling rather than a push/immediate-revocation claim. Local mutations clear
immediately through shared query invalidation.

On a failed detail refetch, hide cached payload instead of displaying it beside
the error. Exact evidence detail polls at four seconds, consumes cancellation and
clears its body on expiry; the inspection offers an explicit refresh. Claim detail
continues its existing canonical polling and hides stale payload on error.
Dismissed inspections and superseded search requests consume cancellation signals;
late responses cannot repopulate the new selection. Cancellation of a dispatched
provider request does not promise cancellation of provider cost.

No persistence of queries/context, automatic semantic replay, new retry policy,
second search endpoint or mutation implementation is introduced. Only an explicit
Recall submission creates a query-model attempt. Context copy reports clipboard
failure and only copies the bounded attributed response; source and comparison
inspection cannot silently add data to it.

## Acceptance

- Real desktop flow: mixed current/disputed records, exact source versions,
  disagreement comparison, strict exclusion with a positive accepted control,
  qualified historical revision, optional review and current graph drill-through.
- Retained procedure/handover inspection keeps its existing conditions/outcomes/
  contributor data; search and review never authorize execution.
- Same Brain/scope/manifest/fact-time graph requests and exact revision keys;
  no historical graph substitution, no hidden model request or omitted-claim fan-out.
- No-match, insufficient-support, missing bytes, partial coverage, permission
  denial, stale/error/expiry and in-flight filter/Brain changes have usable states.
- Actual API deadline fixtures with retained positive controls; browser failures
  hide old payload, evidence expiry clears independent of its parent, remote
  epoch changes clear text-only context, and optional corrections invalidate it.
- Existing recall/review/graph browser regressions and focused Rust/API/web checks,
  normal runtime preservation, a verified runbook and `./scripts/validate.sh`.

## Explicit Deferrals

Automatic assertion of answer sufficiency, cross-query saved investigations,
historical graph reconstruction, mobile, MCP host injection and execution UI.
Generated answers are owned by the separate accepted answer contract, not by
these search views. Search does not promise exhaustive conflict/source discovery.
