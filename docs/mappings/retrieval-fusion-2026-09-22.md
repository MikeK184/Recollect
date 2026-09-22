# Retrieval fusion reference and implementation evidence, 2026-09-22

Observed: 2026-09-22
Confidence: verified locally, with the evaluation limits below.

## Method and boundaries

Read current retrieval/graph code, accepted contracts and local Atlas/Cognee
references. The user-requested independent reference agent read the six relevant
Atlas patterns and Cognee retrieval paths without edits, providers or tests.
Reference checkout Cognee remains at `c0d18c80e24b7b78918e7642c03f6f128fdd2aee`.
This mapping is evidence; the [contract](../contracts/retrieval-graph-fusion.md)
governs implementation. Mobile is deferred by explicit user direction.

## Findings and decisions

- Atlas `hybrid-retrieval-fusion`: one canonical identity and one contribution
  per channel; preserve exact priority and measure ablations.
- Atlas `source-diverse-context`: coverage then depth, with a bounded preference
  rather than one-record-per-file. Source diversity does not prove independence.
- Atlas `gate-the-expensive-path`: no qualified anchors skips native traversal;
  no paid reranker or GDS projection is needed for this channel.
- Atlas `evidence-before-belief`: target provenance and discovery witness remain
  distinct. Path existence never manufactures acceptance or supporting evidence.
- Atlas `retrieval-hysteresis` and `cache-preserving-injection`: preserve the
  existing foundation deferral; no suppression of corrections or fresh scope.
- Cognee `modules/retrieval/hybrid/pairs.py` joins chunks/summaries through
  `source_chunk_id`; `entities.py` deduplicates relation bullets and caps per
  entity. `hybrid/merge.py` and `utils/merge_results.py` reserve query-rewrite
  lanes, not independent source families. `hybrid/ranking.py` ranks chunk IDs;
  `CogneeGraph.py` scores vector distances/optional weights. None of the inspected
  paths supplies a stronger canonical source-lineage policy for Recollect.
- Reuse Recollect's graph selector, complete verification and prefiltered native
  reachability, extending witness validation for bounded candidate prefixes.
  The existing source qualification examines all fragments before vertex use.
  Knowledge support edges are exact-revision projections; public historical
  graph retrieval is unavailable until a separate historical design exists.
- Source grouping uses all direct supports. Exact-version excerpt lineage and
  original capture binding/session/agent metadata exist already. Parent expiry
  may leave an independently retained excerpt; only opaque identity is used.
  There is no existing nesting bound, so the new optional lookup is explicitly
  bounded and gives unknown ancestry no novelty credit.

## External interface evidence

Context7 resolved `/websites/neo4j_cypher-manual_current`; its shortest-path query
returned general examples rather than complete prefilter evidence. The
[official shortest-path manual](https://neo4j.com/docs/cypher-manual/current/patterns/shortest-paths/)
and actual native probes in [slice 19](graph-exploration-interfaces-2026-09-22.md)
establish `SHORTEST 1`, prefilter placement and bounded selection for installed
Neo4j 2026.08.1. Equal-length witness ties are not deterministic.

The original [Cormack, Clarke and Büttcher RRF paper](https://cormack.uwaterloo.ca/cormacksigir09-rrf.pdf)
uses a sum of reciprocal ranks with constant 60. Recollect extends its existing
reducer using this fixed baseline; paper results are not product quality proof.
No new engine/package/version pin or content hash is introduced.

## Local acceptance

Delivered the [contract](../contracts/retrieval-graph-fusion.md) through the
existing recall handler, `graph/recall.rs`, `retrieval/context.rs`, native scope
commands and desktop `RecallPanel.tsx`. No migration, second graph engine or new
model operation was added. The graph adapter reuses Neo4j native reachability;
canonical records govern every input and returned witness.

- Full platform run: **98 passed**, zero failures, **305.11 seconds**, with
  unrelated live OIDC excluded. Two additional platform cases passed afterward:
  the 100-candidate graph cap (1.17 s) and requalification across an actual
  provider wait (4.47 s). These are 100 passing scenarios in a composed proof,
  not a claim that the final 100 were rerun as one suite.
- Real PostgreSQL/Neo4j fixtures cover direction, hop limits, cycles, parallel
  edges, self loops, one canonical graph vote, exact priority, semantic scoring
  span preservation, candidate bounds, insufficient anchors, complete context
  budgets, physical projection damage and qualified alternative routes around
  a withheld shorter hub. A withheld whole-source vertex cannot be seeded by
  an individually eligible recalled fragment.
- Paired native recall preserves its immutable operation selection after the
  task default changes; forged scope is denied. Unsupported graph history and
  damaged physical input refuse before a query-model request is dispatched.
  Provider-wait erasure removes the victim; a newly published claim stays out
  of the frozen query and appears in a fresh positive-control query.
- A delayed native proxy proves the aggregate 15-second graph budget spans
  preflight, requalification and expansion while excluding the provider wait.
  Six individually permitted 2.6-second graph delays still time out as a whole.
  The paid attempt is recorded once and is not replayed; a fresh explicit query
  succeeds. Shared combined-input proof covers hidden-input expiry across a
  deliberately delayed shortest-path call, with policy extension as a control.
- Context fixtures cover all direct supports, nested retained excerpts, parent
  expiry, cycles, explicit descendant Erase, original capture binding/session/
  agent identity and later ordinary versions. Unknown lineage remains eligible
  for rank fill without novelty credit. A smaller qualified baseline fragment
  fits where its earlier larger alternative cannot. The coverage fixture adds
  an independent root (one to two) while preserving four results and useful depth.
- Workspace tests: **13 passed**. Clippy workspace/all-targets with denied
  warnings passed. OpenAPI generated **125 operations**; browser typecheck and
  build passed. Governance validation and **32 checker tests** passed.
- Desktop browser flow: **passed in 9.6 seconds**. Real graph witnesses and
  evidence inspection, inert source text, invalid graph-only selection, scope
  clearing, request failure, deadline expiry and actual rejection-epoch clearing
  are exercised. No filter or invalidation automatically repeats paid recall.
  The synthetic and normal SWEG desktop screenshots were visually inspected.
  Mobile was excluded following the user's explicit direction.

The independent read-only source review found three issues during development:
reset cumulative graph budget, a second awaited clock read after the whole-graph
expiry gate, and loss of baseline fragment fallback. All were repaired before
closeout, with the focused cases above. Physical preflight also now precedes
admitted paid query dispatch. This review is evidence, not a new approval gate.

## Fixed-corpus actual-model comparison

The checked-in `fusion-corpus.json` adds ten declared claims and eight fixed
questions to the seven synthetic documents in `semantic-corpus.json`. Four
questions identify relationship anchors, two are paraphrases and two are
unsupported. Each supported question names a target claim in advance. Each lane
uses four result slots and the same 8,192-byte attributed-context budget. No
customer content or text-generation model was used.

The following table uses the default source-coverage preference. The same lanes
were also run with that preference disabled; target hits, unsupported returns,
irrelevant counts and median context sizes were identical on this small corpus.

| Channels | Target hits / 6 supported | Unsupported questions returning context / 2 | Irrelevant records across 8 questions | Median / maximum latency | Median context bytes |
| --- | --- | --- | --- | --- | --- |
| Exact + lexical | 0 | 0 | 0 | 26 / 30 ms | 941.5 |
| Exact + lexical + graph | 4 | 0 | 0 | 112 / 131 ms | 3,265.5 |
| Exact + lexical + semantic | 3 | 2 | 17 | 405 / 943 ms | 5,334 |
| Exact + lexical + semantic + graph | 6 | 2 | 14 | 538.5 / 618 ms | 6,967.5 |

The baseline found the four exact anchors, but not their predeclared associated
target claims; zero target hits does not mean it found no useful evidence.
Graph addition alone missed the two paraphrases because they supplied no
eligible anchor. Semantic plus graph found all six targets, while still returning
unrelated context for both unsupported questions. This is retrieval evidence,
not an answer-generation or reliable-abstention result. No general superiority,
independent corroboration or diversity-quality improvement is claimed.

All **64 queries** completed against a separate owned database with the selected
**`text-embedding-3-large`, 3,072 dimensions**. Provider usage records contain
**33 requests**: one batch embedding 17 corpus records and 32 query embeddings.
Total charged input was **1,227 tokens** (975 indexing, 252 queries); no monetary
price is inferred. Preference-disabled median latencies were respectively 27.5,
109, 421.5 and 587 ms; maxima were 36, 220, 483 and 704 ms. These local single-run
measurements do not establish a general latency bound or a ranking advantage.

`scripts/evaluate-retrieval-fusion.mjs` admits only the isolated loopback proof
listener, persists every paid attempt before dispatch and refuses automatic
replay when its state file exists. Completed state, summary, logs, database dump,
artifacts and privacy journal remain in ignored `.cache/fusion-live-proof/`.
The proof API/worker stopped after completion; its separate database is retained.
Do not repeat it as an ordinary test or infer that its model calls occurred in
the normal application database.

## Normal retained SWEG runtime

The existing exact repository snapshot and environment manifest returned **99
qualified nodes / 183 edges**. A two-hop graph recall around the exact
`var.ssh_public_key` fact found **73 connected candidates** and returned six
attributed records in **15,735 wire context bytes**, in **152 ms**. All repository
provenance retained the same exact commit. Five source groups were resolved with
no unknown lineage. Coverage remains partial for shortened fragments, unresolved
extraction targets and the result limit. A separate desktop submission exercised
the visible controls and evidence witnesses with its own ten-result UI budget.

Before/after checks match all **seven Brain IDs, 33 model requests, zero active
jobs and migration 020** byte for byte. This proof added **zero model calls and
zero customer file reads**. The normal API and worker remain available locally
at `http://127.0.0.1:8787`. This is local runtime evidence, not deployment proof.

Ignored evidence includes `.cache/retrieval-fusion-platform.log`, the focused
`graph`, `semantic`, `context`, `capture`, `budget`, `candidates` and
`requalification` logs, `retrieval-fusion-ui.log`, `retrieval-fusion-runtime.json`,
the preservation JSON pair and both desktop screenshots. Raw wire context bytes
are checked before JavaScript parsing because reserialization can shorten an
integral floating-point value without changing its meaning.
