# Scoped graph fusion and source-aware context

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Retrieve recorded relationships alongside exact/lexical/semantic evidence
  in one qualified, attributed, bounded context.
- Non-goals: Historical graphs, new engines, generated answers, learned ranking,
  persistent result caches, host injection and mobile views.
- Delivery shape: Local Rust API/native client, desktop React controls and proof.

## Governing Sources

- [Fusion contract](../../../contracts/retrieval-graph-fusion.md)
- [Baseline recall](../../../contracts/retrieval-exact-and-lexical.md)
- [Semantic recall](../../../contracts/retrieval-semantic.md)
- [Graph exploration](../../../contracts/graph-exploration.md)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: Opt-in native graph candidates, shared fusion, bounded canonical
  lineage preference, attribution, API/native/desktop consumers and evaluation.
- Out of scope: Separate graph/vector engines, scope mutation, GDS per recall,
  customer corpus transmission or mobile work.
- Blockers: None; predecessor slices are shipped and routine decisions resolved.

## Surface and Interface Changes

- Interfaces: Extend existing recall types and route with contract options,
  graph witnesses/status and context selection metadata; generated OpenAPI.
  Extend native recall flags and desktop filters/results without another handler.
- Storage: No migration; canonical source/capture/excerpt rows and graph
  generations supply request-local metadata only.
- Ownership: Graph owner exports a bounded internal recall adapter using its
  selector/verification/native reach/final gate. Retrieval owns fusion and final
  context admission; canonical qualification remains the shared read boundary.

## Data and Authority

- Inputs: One immutable parent selection/cutoff, eligible ranked anchors,
  qualified graph and exact canonical source lineage metadata.
- Authority: Current caller RLS, Brain lock, canonical memory/retention policy.
  Native graph membership and paths are verified against that authority.
- Blind spots: Missing/stale projections, unsupported extraction, bounded
  candidates/lineage, unavailable history and no-match do not prove absence.

## States and Edge Cases

- Loading: Explicit query only; disable duplicate paid submission.
- Empty: No anchors/no neighbors/no match/insufficient support remain distinct.
- Error: Graph channel failure refuses the request; old UI context clears.
- Blocked: Invalid history/kind/scope/options and size/capacity/timeout give
  safe actionable errors; validate before paid embedding.
- No-access: Reuse parent operation access and recheck after provider wait.
- Duplicate or replay: One identity and graph vote; no automatic model replay.
- Stale data: Frozen knowledge cutoff, current corrections/erasure, complete
  graph deadline guard and browser scope/epoch/expiry invalidation.
- Reconciliation divergence: Physical graph and witness validation refuse
  divergence. Unknown lineage gives no novelty credit, with rank fill preserved.

## Integrations and Runtime Inputs

- Providers: Existing Neo4j Query API, PostgreSQL/pgvector and approved embedding
  gateway; no new dependencies or mandatory Cognee runtime.
- Environment: Existing PostgreSQL, Neo4j and `OPENAI_API_KEY` settings.
- Secrets: Existing ignored environment files; no values in fixtures or output.
- Failure handling: Two graph slots, one aggregate 15-second graph phase,
  30/90-second recall bounds, two-second SQL and existing native timeout;
  no implicit provider retry/fallback.

## Tests and Acceptance

- Automated: Meaningful real PG/Neo4j fusion, qualification/lineage/expiry and
  failure fixtures; paired native and desktop controls/witness/clearing proof.
  Run workspace, Clippy, generated API, web build and governance validation.
- Manual: Inspect desktop proof and normal retained SWEG relationship query;
  record fixed synthetic channel/diversity ablations with actual model evidence
  where semantic quality is claimed. Preserve existing Brains/data/services.
- Acceptance: All contract cases and measured limitations recorded in the
  [dated mapping](../../../mappings/retrieval-fusion-2026-09-22.md).

## Closeout

- Planned: Complete current-scope graph fusion, source preference and consumers.
- Shipped: Native qualified graph discovery and witness validation, shared
  exact/lexical/semantic/graph fusion, bounded source-lineage coverage/depth,
  provider-wait requalification and aggregate graph budget, desktop controls
  and paired immutable-scope CLI. Actual synthetic model comparison and retained
  SWEG desktop/API proof are recorded in the dated mapping.
- Not shipped: Historical graph projections, learned ranking/gates, ANN,
  generated answers, query rewriting, hysteresis, persistent caches, host
  injection and mobile views; these remain explicit contract deferrals.
- New blockers: None.
- Docs updated: Contract, dated mapping, [runbook](../../../runbooks/graph-recall.md),
  execution pack, owning epic and lifecycle indexes, README and continuation note.
- Validation: Full platform run 98 passed; two subsequently added scenarios
  passed separately (100 scenarios in composed proof). Final semantic-span and
  graph cases passed. Workspace 13, Clippy, formatting, OpenAPI 125 operations,
  web typecheck/build, desktop browser and governance 32 tests passed. Actual
  model proof completed 64 fixed-corpus queries with 33 embedding requests and
  1,227 charged tokens in an isolated database. Normal SWEG recall found 73
  connected candidates; all seven Brains and 33 existing model requests were
  preserved, with no new customer reads or model calls in that runtime.
- Version: N/A: no release policy.
- Commit: Uncommitted.
