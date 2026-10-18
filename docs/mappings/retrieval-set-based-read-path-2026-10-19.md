# Set-based recall read path: measurements and plans

Observed: 2026-10-19
Confidence: observed-once

## Sources and Method

Local PostgreSQL 17 (pgvector image used by the repository compose stack) and the
platform test harness, run against disposable databases created by
`crates/server/tests/platform.rs`. The previous code was built from the checked-out
parent commit in a separate worktree and run with the same test file. Statements are
counted by a tracing layer on the `sqlx::query` target
(`crates/server/tests/platform/retrieval/statements.rs`); counts include
transaction control statements. Plans come from `EXPLAIN` of the window queries and
`pg_stat_user_indexes.idx_scan` deltas around a real HTTP recall. Nothing here was
run against the installed normal-data Brain, and no release or deployment occurred.

## Observations

- Statements per recall with 100 candidates (85 source fragments and 15 accepted
  claims, one lexical query): previous code **350 lexical / 351 automatic**; set-based
  code **26 / 27**, about 13× fewer. The previous count grows with every candidate
  (per-claim support, rule, conflict, deadline and scope statements plus per-row
  raw-rule and scope statements); the new count is fixed by the number of gate
  categories, not by the candidate count.
- Row-level security (the app role) makes `tsvector @@ tsquery` and jsonb literal
  extraction non-leakproof barrier quals, so the `recall_vector` GIN indexes cannot
  be index conditions on the app role's own scans even when the predicate is moved
  into `WHERE`: the planner showed a sequential scan with a filter on a 6,000-row
  claim table while the same predicate as the migration role used the GIN index.
  Migration 048 therefore resolves candidate identifiers in `SECURITY DEFINER`
  window functions (membership test first, identifiers only) and the candidate
  chain re-reads those rows through RLS.
- With 20,000 padded rows per table, the window queries plan as bitmap scans on
  `claim_recall_lexical`, `source_recall_lexical`, `fact_recall_lexical` and
  `manifest_recall_lexical`; `source_recall_title` is eligible (chosen with
  sequential scans disabled) but a narrow table with short high-cardinality title
  vectors is legitimately cheaper to scan. A recall through the HTTP API as the app
  role incremented `idx_scan` of the four indexes above.
- `recollect_memory_supported` costs about 25–50 ms per revision for the app role
  (about 0.8 ms as the migration role) on the test database. A 60-claim recall took
  over the 2-second statement limit with the previous code (the Brain-wide chain ran
  several times) and with a single pass; the 15-claim fixture is the largest that
  fits comfortably. The single pass evaluates it once per windowed claim; it is no
  longer re-asked by the claim gate or the conflict query.
- Existing platform results (new code vs. parent commit, same database host):
  `retrieval::` 9 pass / 3 fail on both; `models::` 99 pass / 4 fail on both;
  `memory::`, `review::`, `retention::` the same 5 failures on both. Every failing
  test fails at the same line on both builds (earlier fixtures that treat unreviewed
  proposals as usable support, which the source-support gate no longer allows).

## Translation and Limits

- The failing tests above are not caused by this slice, and they do not exercise
  claim eligibility through recall, so conflict and rule equivalence rests on the
  passing support, handover, autonomous and procedure suites plus code review; there
  is no stored differential fixture for `conflicts_many`.
- The windows cover the lexical-only request. Semantic scoring and the project brief
  need query-independent populations and use the full candidate pass (once per
  transaction; a semantic request has two transactions).
- Larger populations of claims remain bound by `recollect_memory_supported` per claim;
  that function is outside this slice.

## Follow-up

- Refresh stale platform fixtures to use reviewed or audited claims, then add a
  conflict/rule differential case.
- Measure on the installed Brain after the owner approves a migration 048 update.
- Consider a set-returning or cached form of `recollect_memory_supported`.
