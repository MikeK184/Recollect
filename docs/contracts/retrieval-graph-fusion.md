# Scoped graph retrieval and source-aware context

Status: accepted

## Source

The full 29-slice product goal authorizes routine decisions and implementation.
The accepted [retrieval baseline](retrieval-exact-and-lexical.md),
[semantic recall](retrieval-semantic.md), [graph exploration](graph-exploration.md),
[combined graph](graph-cross-repository-views.md),
[retention](memory-retention-and-erasure.md) and
[engineering principles](../foundation/engineering-principles.md#combine-retrieval-methods-under-one-contract)
govern. The [dated reference audit](../mappings/retrieval-fusion-2026-09-22.md)
records Atlas, Cognee and native interface evidence. Desktop acceptance follows
the user's 2026-09-22 instruction; mobile work is deferred.

## Contract

### Opt-in graph discovery

Add `graph` to recall channels. Existing defaults remain exact plus lexical.
Graph requires at least one other selected channel to supply query anchors;
graph-only requests are invalid. Optional `graph` options select `kind`
(`knowledge` default, `repository`, `combined`), `direction` (`both` default,
`incoming`, `outgoing`), `max_hops` (default two, one through three), and up to
20 supported `relations` (empty means all qualified relations). Options without
the channel are invalid. This is recorded connectivity, not inferred impact.

Inherit the parent recall's Brain, actor/device/operation, collection, repository,
area, environment, manifest, fact-time and eligibility mode. There is no second
graph scope or snapshot knob that can widen or contradict the parent. Repository
graphs require one selected repository and use its exact manifest entry, or its
latest snapshot known at recall admission without an environment. Combined graphs
require the parent's explicit environment and manifest. Knowledge uses its ready
projection with canonical requalification. Missing/stale/partial input retains
the graph owner's errors and coverage. No graph engine fallback is implicit.

Public `knowledge_at` and history mode are unavailable with the graph channel:
reject before any paid query attempt. Current recall freezes one internal
knowledge cutoff at admission, shared by all channels and repository resolution.
Current corrections, erasure, retention and access still override that cutoff.
Prepare canonical graph bounds and projection availability before embedding;
verify physical input before any admitted query-model call. Verify again after
the provider wait. Both preparations/verifications and native expansion consume
one cumulative graph budget; provider wait itself is excluded from that budget.
Release locks for the provider as today; reselect and requalify after it returns,
using the original cutoff. A later projection cannot manufacture a historical
edge: knowledge support relations must belong to the qualified exact revisions.
Missing older projected vertices are explicit partial coverage.

Choose at most three ranked anchors only after intersecting the fully qualified
graph. A recalled safe fragment whose whole source vertex is withheld cannot
seed traversal. Preserve exact/literal priority, then existing lexical/semantic
ranking with stable identity ties. No eligible anchors skips native traversal and
reports `no_eligible_anchors`; this is a cheap deterministic gate, not a learned
classifier. It creates no provider request or GDS report of its own.

Reuse native prefiltered `SHORTEST 1` reachability over the complete qualified
input (5,000 candidate rows/nodes, 20,000 edges and existing descriptor bounds).
Verify selected physical projections once before traversal or claiming no
neighbors. All anchors share one of the existing two graph read slots and one
15-second graph phase, including qualification and verification. Each native
call retains its existing timeout. With graph enabled, total recall allows
30 seconds without semantic and 90 seconds with semantic; SQL remains two
seconds. Capacity, timeout, unavailable or malformed projection refuses the
whole recall; never silently omit a requested channel.

Each anchor fetches up to 101 targets, ordered by distance and exact vertex key.
One hundred is the per-anchor candidate bound; overflow is explicit coverage.
Validate every returned witness's endpoints, adjacency, direction, membership,
unique vertices/edges and hop bound, including intermediate entities that are
not themselves candidate targets. Exclude every anchor from graph contribution
(no zero-hop/self or mutual-anchor boost). Union by canonical identity, selecting
the shortest witness, then earliest anchor. Select at most 100 graph identities,
ranked by distance, anchor order and canonical kind/ID. Equal-length native
witness ties may vary; the ranking cannot depend on the tied route. Multiple
paths, parallel edges, cycles and hubs never produce extra votes for one identity.

### One reducer, explicit attribution

Extend the existing identity-priority reciprocal-rank reducer: exact/literal
matches first, then sum `1/(60 + channel_rank)` for lexical, semantic and graph
arms, with stable kind/ID ties. Preserve existing cosine-only and baseline
algorithms when graph is absent. A canonical result appears once. A semantic
match retains its actual scoring fragment and similarity when graph also finds
it; graph qualification must not replace that span with a first source fragment.
Keep bounded qualified baseline fragment alternatives for byte-budget admission
when that identity has no semantic score. Label an alternate selected for budget;
a scored semantic span cannot be silently substituted to fit.

Each graph-matched result carries the selected generation, exact anchor and
witness entities (key, kind, ID, revision and label), direction, and canonical
relationship records with their evidence identities. Its own provenance remains
separate. Add `graph_proximity_not_truth`; connectivity supplies neither support
nor acceptance, independence, deployment state or execution authority. Full
attributed witnesses participate in the existing serialized context-byte budget.
Omit an item that does not fit; do not remove qualifications/provenance to fit.
Response graph metadata records exact selection/generation, coverage, anchors,
candidate count and earliest selected-input expiry. Final graph authorization
and retention checks cover the complete selected input, including hidden nodes
that influenced shortest paths, immediately before returning context.

### Bounded coverage preference and depth

Add `source_diversity` (default true) to all recall requests. This refines final
context admission, after qualification and ranking, without changing scores.
False permits controlled ablation. Admit fitting exact-priority items first.
Then reserve up to half the requested result slots (rounded up, including those
already admitted) for ranked items adding an unseen canonical source group.
Fill remaining slots from global rank, allowing neighboring facts or several
useful claims from the same source. Return admitted items in original rank order.
No hard per-file quota, text hashing or independence claim is introduced.
Update covered groups only after the entire attributed item fits the budget.

Group source versions by their canonical source; follow excerpt parent **exact
versions** through same-Brain `source_excerpts` and consistent version/source
pairs. Original captured versions may group by Brain, binding, validated host
session and nullable agent ID, only for effectively accepted/unexpired capture
metadata. Later ordinary versions do not inherit original capture session identity.
Repository facts group by repository, exact snapshot and canonical path where
present, otherwise exact fact identity. Manifests group by exact revision. Claims
use the set of all direct evidence roots; one shared support is not a new source.

Resolve only the at-most-300 candidate union's referenced metadata under the
current Brain lock and caller RLS. At most 6,000 distinct direct references,
eight excerpt-parent steps and 8,192 total version rows are inspected in batches.
Cycles, inconsistent/missing links or bounds produce unknown lineage, without
novelty credit; the eligible item remains available to rank-based fill. No full
Brain scan, source text read or public ancestor/session identifier is added.
An expired parent's opaque source identity can still group its independently
retained excerpt. It supplies no bytes, inherited TTL or traversable bridge.
Explicit Erase retains the existing descendant-removal behavior.

Expose selection metadata: whether preference was enabled, number of distinct
resolved groups in returned context and number of returned items with unknown
lineage. Coverage preference is not independent corroboration. Request-local
metadata is discarded with the transaction; no recall/vector/graph result cache
or hysteresis is added. Browser clears context on scope/filter/access/epoch
change and at graph expiry; paid queries are never automatically replayed.

### Consumers

Extend desktop recall with graph channel/kind/direction/hop/relation controls,
coverage preference, clear incompatibility messages and readable graph witnesses
beside canonical evidence. Keep original evidence inspection and graph exploration
available. Native `scope recall` accepts channel, manifest, graph kind/direction/
hops/relations and coverage-preference options while retaining its immutable
bound operation scope and one new semantic request ID per explicit invocation.
The combined investigation workflow remains slice 21; basic current-slice
controls, errors, provenance and clearing are delivered here.

## Acceptance

- Actual PostgreSQL/Neo4j relationship recall with positive unrelated controls,
  all directions, hop bounds, duplicate paths and qualified alternative routes.
- One canonical result and graph vote across channels; semantic scoring-span
  preservation; complete byte budgets, result/candidate/input bounds and explicit
  no-anchor/partial/unavailable states with no hidden provider charge.
- Scope/operation/manifest isolation, strict/history behavior, blocked raw source
  fragments, corrections/erasure/projection damage, retained excerpt ancestry,
  and permission/retention changes across provider/native waits.
- Multi-support claims, nested excerpts, capture binding/session/agent separation,
  independent positive roots and useful depth retained in the same budget.
- Fixed synthetic questions compare exact/lexical, semantic additions, graph
  additions and source preference under equal budgets. Report hits, irrelevant
  results, misses, latency and provider usage without claiming general superiority.
- Desktop/browser and paired native API proof, required Rust/API/web checks,
  normal-runtime preservation, runbook and `./scripts/validate.sh`.

## Explicit Deferrals

Historical graph projections, learned ranking/gates, ANN, inferred links, generated
answers, query rewriting, retrieval hysteresis, persistent caches, host injection
and mobile views. Existing privacy/correction/retention gates are not deferred.
