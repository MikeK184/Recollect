# Recall recorded relationships

## Prerequisites

Start `./scripts/dev.sh`, open `http://127.0.0.1:8787`, and select a Brain you
can read. Its selected knowledge or repository graph must be ready. See
[graph projection](graph-projection-and-traversal.md) and
[combined views](graph-cross-repository-views.md) for rebuild and exact-manifest
selection. Exact/text plus graph recall needs no model call. Semantic additions
require the standing [model and embedding policy](semantic-recall.md).

## Desktop procedure

In **Recall memory**, expand **Scope, time and exact lookup** and select graph
alongside exact, text or semantic search. Graph discovers recorded neighbors of
up to three qualified query matches; graph-only search is unavailable. Choose
knowledge, repository or combined graph, direction, one to three hops and any
relationship filters. The graph inherits the same Brain, collection, scope,
manifest, fact time and eligibility mode as the rest of the query.

Repository graph requires one selected repository. With an environment, select
its exact manifest; without one, the latest snapshot known when the request
starts applies. Combined graph requires both environment and manifest. Historical
mode or an explicit knowledge timestamp cannot be combined with graph yet.
The UI explains these combinations before submission.

Keep source-coverage preference enabled to reserve some context for additional
canonical source groups, then fill with ranked depth. It does not promise
independent corroboration or impose a one-result-per-file limit. Results show
the number of resolved groups and unknown-lineage items. Disabling preference
allows an explicit comparison using the same query and budget.

Press **Recall**. Graph results show the anchor, recorded path and relationship
families beside the result's own evidence links. A path explains discovery;
it is not supporting evidence, acceptance, deployment state or permission to
execute a procedure. **Inspect evidence** opens canonical retained evidence.
Use the Brain's graph explorer for the wider bounded graph or a path query.

The full attributed context, including witnesses and provenance, must fit the
selected byte budget. A large witness may leave fewer results; increase the
explicit budget or narrow the query. Copying context includes its data-only
instruction. Discard any external copy when scope or authority changes.

Scope/filter changes, graph invalidation and expiry clear displayed context.
Failed refresh also removes prior results. These events do not silently repeat
a paid semantic query; submit a fresh query explicitly when appropriate.

## Native procedure

Pair the native companion and begin a context operation using
[workspace scope](workspace-scope.md). Use the returned immutable operation ID:

```sh
./target/debug/recollect-agent scope recall "$BRAIN_ID" "$OPERATION_ID" "Vault JWT" \
  --channels exact,lexical,graph --graph-kind knowledge \
  --graph-direction both --graph-hops 2 --source-diversity true
```

`--manifest ID` supplies the parent exact manifest; `--graph-relations` accepts
a comma-separated list of supported relation names. Repository/environment
selection comes from the bound operation, even after the task default changes.
Begin a new operation when that selection should change. Adding semantic creates
one fresh query attempt per explicit invocation through the same API handler.

## Failure and limits

- No eligible anchors: check exact identity, query text and scope; a safe recalled
  fragment cannot seed a whole-source vertex withheld by current qualification.
- No neighbors: a verified graph contains no connected qualified candidate under
  these direction/relation/hop settings. This is not proof of real-world absence.
- Missing/stale/damaged graph: inspect graph status and rebuild through its owner.
  A requested graph channel failure refuses the whole query without a silent
  fallback. Physical availability is checked before a paid query embedding.
- Partial graph: coverage names omitted projection inputs or the candidate bound
  (up to 100 discovered identities). Narrow scope/query; paths never widen scope.
- Input/capacity/timeout error: complete input is bounded to 5,000 vertices and
  20,000 edges, with two graph read slots and a cumulative 15-second graph-work
  budget. Total recall allows 30 seconds with graph or 90 with semantic. Provider
  wait does not consume graph-work time; no automatic model retry is added.
- Insufficient support: inspect claim qualifications or use the strict modes.
  Semantic similarity and connectivity do not establish support for an answer.
  Unsupported synthetic questions still returned unrelated semantic context in
  the [measured comparison](../mappings/retrieval-fusion-2026-09-22.md).

## Verification

With the repository-owned services available:

```sh
set -a
source .env
set +a
cargo test -p recollect-server --test platform graph::retrieval:: -- --ignored --test-threads=1
cargo test -p recollect-server --test platform retrieval::context:: -- --ignored --test-threads=1
./scripts/test-ui.sh tests/recall-graph.spec.ts
./scripts/validate.sh
```

These fixtures use synthetic inputs and controlled providers; no actual OpenAI
charge is required. The actual-model evaluation script is a separate, explicitly
isolated proof with retained attempt state, not a routine test or an automatic
replay procedure. The mapping records its measurements and the normal SWEG
runtime preservation check. Current UI verification targets desktop only.
