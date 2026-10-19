# Semantic recall over large scopes

Status: in-progress
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product
Implementation: authorized by the repository owner on 2026-10-19 (design final; the owner accepted a labelled approximate ranking above the exact bound instead of a refusal)

## Summary

- Goal: Semantic recall works for every scope up to the existing 50,000
  entries per Brain cap inside the two-second statement limit. Scopes up to
  5,000 representations keep today's exact cosine ranking. Larger scopes get a
  labelled approximate ranking: a bit-signature shortlist, then exact cosine
  re-ranking of the shortlist. Auto mode keeps semantic search for those scopes
  instead of dropping it.
- Non-goals: A vector index of any kind (HNSW or IVFFlat), changing what is
  eligible, ranking or fusion (RRF k=60), the query embedding path, model
  policy, or the 50,000 entry cap.
- Delivery shape: One migration, the two recall query shapes, honest coverage
  fields and reason codes, an evaluation flag, contract amendments and
  measured recall/timing tests.

## Governing Sources

- [Governed semantic representations and recall](../../../contracts/retrieval-semantic.md)
- [Brain model catalogue and selection](../../../contracts/brain-model-catalogue-and-selection.md)
- [Canonical exact and lexical recall](../../../contracts/retrieval-exact-and-lexical.md)
- [Technology stack: storage and hybrid retrieval](../../../foundation/techstack.md#storage-ownership-and-hybrid-retrieval)
- [Engineering principles: retrieval](../../../foundation/engineering-principles.md#combine-retrieval-methods-under-one-contract)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: Migration `072_semantic_signature.sql` (a `signature bit varying`
  column kept by the existing dimension trigger, a bounded backfill and a
  consistency check); the exact-tier query rewrite; the shortlist tier; the
  `semantic_exact` request flag; `RecallSemantic.method`, uncapped
  `scoped_entries` and `reranked`; reason codes `semantic_shortlist_approximate`
  and `semantic_shortlist_exhausted`; counts in the partial-coverage notes;
  auto mode no longer dropping semantic search below the 50,000 cap; contract
  amendments; regenerated API types; minimal UI text for the new codes.
- Out of scope: A shared or per-Brain ANN index, half-precision or reduced
  dimensions, query expansion, learned reranking, changing the 50,000 cap or
  the 100-candidate limit, and any web redesign beyond the reason-code text.
- Blockers: None. The owner decided the design on 2026-10-19.

## Decisions

- Diagnosis: Exact cosine runs over vectors stored out of line (TOAST, 12 KB at
  3,072 dimensions). The scoped CTE fetches each vector through a per-row
  lateral lookup and the scoring statement then evaluates
  `embedding<=>$3::text::vector` twice per row, casting the query text on every
  evaluation. The 5,000 bound guards the two-second statement limit.
- Rejected: A shared HNSW or IVFFlat index breaks Brain isolation (other
  Brains' vectors influence results and speed), selective scopes exhaust
  iterative-scan budgets, erased vectors linger until VACUUM, ordered index
  scans under row-level security are unproven, and IVFFlat stops at 2,000
  dimensions. A per-Brain partitioned HNSW needs runtime DDL and about 300 MB
  of index per full Brain against the 1 GB database memory limit. Both stay
  deferred; the contract records these reasons.
- Exact tier (at most 5,000 scoped rows): identical ranking and results.
  The query vector is parsed once in a `q` CTE, the distance is computed once
  per row, ordering is `d,kind,id,chunk_id NULLS FIRST`. The transaction sets
  `jit=off` and runs `ANALYZE recall_pool` after the temporary pool is built,
  because temporary tables have no statistics. The 5,000 bound stays unless
  measurement justifies another number; it is not raised without recorded
  timings.
- Shortlist tier (more than 5,000 rows, up to the 50,000 entry cap): the scoped
  filter selects identities plus `e.id` and `e.signature` only (no vector).
  `shortlist` materializes the best M rows by Hamming distance
  `signature <~> binary_quantize(q.v)`, ordered `h,kind,id,chunk_id NULLS
  FIRST`, M a code constant (2,000, tuned by measurement; profiles with fewer
  than 512 dimensions use the exact tier regardless of scope size when the
  measurement shows it fits, otherwise a larger M). The shortlist joins back to
  `recall_pool` and `semantic_entries` and is ordered by the exact cosine
  `e.embedding<=>q.v` with the same tie-break. Isolation holds by construction:
  eligibility and scope filtering happen before any distance, over the Brain's
  own rows. If the Rust loop exhausts the shortlist before 100 identities it
  fetches one more keyset page, then reports `semantic_shortlist_exhausted`.
- Migration 072: `ALTER TABLE semantic_entries ADD COLUMN signature bit
  varying`; `recollect_semantic_dimensions` also sets `NEW.signature` from
  `binary_quantize(NEW.embedding)` (NULL when the embedding is NULL), so every
  writer, including erasure that nulls the embedding, keeps it consistent; a
  `CHECK((embedding IS NULL)=(signature IS NULL))` added NOT VALID then
  validated. The runner applies every pending migration in one transaction
  with a 10 s statement default, so the backfill runs in bounded keyset batches
  with an explicit `SET LOCAL statement_timeout` sized by the batch, never one
  unbounded UPDATE (the statement triggers on `semantic_entries` receive
  transition tables). Restore and rollback: the column is derived data; a
  backup taken before 072 restores into a database whose migration history is a
  prefix of this executable's, and the migration recomputes every signature.
- Migration ordering: 071 is reserved by another branch. The runner indexes the
  `MIGRATIONS` array and requires the applied names to be a prefix, so this
  branch registers 072 directly after 070. Before the branches merge a database
  migrated with 072 but without 071 is incompatible with the merged executable;
  only disposable test databases are affected, and the merge must keep the
  array in numeric order.
- Honest coverage: `RecallSemantic` gains `method` (`exact_cosine` or
  `binary_shortlist_rerank`), `scoped_entries` becomes the uncapped eligible
  count, and `reranked` counts the exactly scored rows. A shortlist-ranked item
  carries the qualification `semantic_shortlist_approximate`, and the response
  carries the same reason. `semantic_scoped_coverage_partial` notes gain
  counts. The algorithm label stays `pgvector-exact-cosine` only for the exact
  tier; the shortlist tier reports `pgvector-binary-shortlist-exact-rerank`.
- `semantic_exact: bool` (default false) forces the exact tier and keeps
  today's refusal above the bound (`semantic_scope_too_large`) for
  evaluations. It needs the semantic channel outside auto mode, like the
  other semantic options. In auto mode the refusal degrades exactly as before
  to `automatic_semantic_scope_too_large`.
- Admission counts up to the 50,000 cap plus one. A scope above the cap still
  refuses with `semantic_scope_too_large`; it is a defense, since discovery
  never creates more than 50,000 entries per profile. Auto mode drops semantic
  search only for that refusal or when `semantic_exact` is set.

## Acceptance

- Automated: Platform tests for exact-tier equivalence with the existing
  semantic tests, scope and ineligibility isolation under the shortlist,
  erasure leaving `signature` NULL, profile/dimension reindex, serialized new
  fields and reason codes, auto mode on 5,000 to 50,000 scopes; synthetic
  recall@100 of the shortlist against exact on the fixed corpus and on 50,000
  deterministic vectors at 8, 384, 1,536 and 3,072 dimensions; `EXPLAIN
  (ANALYZE, BUFFERS)` as `recollect_app` at 5,000, 20,000 and 50,000 rows; the
  existing semantic and retrieval subsets compared with a baseline taken on
  the same tree before the change; `cargo fmt`, `cargo clippy` as in CI, the
  web typecheck and `./scripts/validate.sh`.
- Manual: N/A; recall and timings are measured by the native tests.
- Acceptance: Exact-tier results unchanged; every tier under two seconds at
  50,000 rows; the shortlist's recall@100 against exact is reported per
  dimension; no out-of-scope or ineligible high scorer is ever returned; the
  contracts state the two tiers, bounds, labels and the still-deferred ANN.
