# Set-based recall read path

Status: shipped
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product

## Summary

- Goal: Make recall read per set instead of per row. A 2026-10-18 repository
  review found that `retrieval_candidates.sql` materializes the whole Brain per
  query and computes the full-text match as a boolean column, so the four GIN
  indexes from migration 015 are never used; the candidate CTE chain is executed
  5–8 times per automatic recall; and every surviving candidate is gated by its
  own `memory::view` (per-support, per-rule and per-conflict statements) plus a
  whole-artifact read per source chunk, all under the shared Brain lock with a
  2-second statement timeout.
- Non-goals: Change ranking (exact priority, `ts_rank_cd(…,32)`, identity-priority
  RRF k=60), eligibility, scope, temporal/manifest, correction, retention, privacy
  or coverage semantics; change API shapes, generated types or the client; add ANN,
  a cache that outlives a request, or a new retrieval channel.
- Delivery shape: Rust-owned SQL and gating code in `recollect-server`, one
  migration, a statement-counting platform test and a recorded query plan.

## Governing Sources

[Exact and lexical recall](../../../contracts/retrieval-exact-and-lexical.md),
[semantic recall](../../../contracts/retrieval-semantic.md),
[graph fusion](../../../contracts/retrieval-graph-fusion.md),
[scoped project brief](../../../contracts/retrieval-scoped-project-brief.md),
[engineering principles](../../../foundation/engineering-principles.md),
[technology stack](../../../foundation/techstack.md),
[epic](../../epics/hybrid-retrieval.md). Evidence only (not authority): the
exact/lexical baseline (retained at checkpoint f2673d7 as `docs/mappings/retrieval-baseline-2026-09-15.md`).

## Scope

- In scope:
  1. Lexical window. For lexical-only recall the SQL candidate arms read only
     rows reachable through the `recall_vector` GIN indexes (plus exact
     identifiers/literals and coverage rows), instead of the whole Brain. The
     window is a strict superset prefilter; the exact `lexical_match` recheck on
     the concatenated vector stays, so results are identical. `ts_rank_cd` is
     computed only for rows that matched.
  2. One candidate pass. Each read transaction builds the candidate set once
     into a transaction-local temporary table; coverage flags, the semantic
     admission count and missing flag, ranked rows, semantic scoring, the
     project brief and continuation read that table instead of re-running the
     CTE chain. A semantic request still has two read transactions (the query
     embedding commits the first), so it passes twice; every other request once.
  3. Set gating. Per-candidate `memory::view`/`memory_evidence::row`/
     `memory_rules::*`/`recollect_memory_deadline`/`selection_valid`/raw-rule
     statements become statements keyed by the set of revisions, with one
     implementation shared by the existing single-claim callers. Source chunk
     bytes are verified with a bounded span read, not by reading the artifact.
- Out of scope: the graph recall channel's own entity-window SQL (it already runs
  once per qualification and keeps its markers), answer-bundle revalidation
  (per original item, unchanged except that it shares the batched gate code), the
  `limited`-coverage text-length scan beyond running it once, new ADR/contract
  decisions, deployment to the installed runtime.
- Blockers: None. No ADR or contract gap: no observable behavior changes.

## Surface and Interface Changes

- Interfaces: None. `RecallRequest`/`RecallResponse`, error codes and coverage
  reason strings are unchanged.
- Storage: Migration 065 adds `recollect_claim_status_eligible(jsonb,text)` (the
  single definition of the claim status predicate used by candidate SQL and the
  project brief) and `recollect_lexical_prefilter(tsquery)` (a monotone relaxation
  of a web-search query used only to find source versions whose title or any chunk
  could match the concatenated vector). It adds any expression index the recorded
  query plan shows is still required. No data rewrite, no new authority.
- Ownership: `retrieval*.rs` and `retrieval_candidates.sql` own candidate
  construction; `memory.rs` owns claim eligibility (a batch `base_views` that the
  single-claim `base_view` also calls); `artifacts.rs` owns span reads.

## Data and Authority

- Inputs: Same canonical rows as today under the same Brain RLS transaction, lock,
  role, scope, collection, manifest, knowledge-time and fact-time inputs.
- Authority: Unchanged. The temporary table lives only in the request's
  transaction (`ON COMMIT DROP`), is written only by the candidate statement and
  is never a second source of truth; the post-embedding transaction rebuilds it
  after re-checking authority, manifest, policy and profile as today.
- Blind spots: A GIN window can only be as exact as its relaxation. The relaxation
  keeps every lexeme of the parsed query, and is skipped (full scan, as today)
  when a pure-negation query could match a vector containing none of them.

## States and Edge Cases

- Loading: N/A: synchronous read; the existing 2 s statement timeout and 4-slot
  capacity gate remain.
- Empty: An empty or stop-word query yields an empty window; exact references still
  work; coverage flags still report non-ready sources and over-limit facts.
- Error: A claim whose support row is missing keeps `support_unavailable`; other
  database errors abort the request exactly as before. Per-claim failures do not
  fail the set.
- Blocked: Rejected-value rules, review rules, conflicts, fences, retention and
  support audit gate rows exactly as today; the NOT-EXISTS-style conflict check
  still applies rule validity/scope overlap in Rust (`memory_rules::overlaps`),
  not a second SQL reimplementation.
- No-access: Unchanged; all reads remain RLS-scoped in the caller's transaction.
- Duplicate or replay: Unchanged semantic attempt rules; the temporary table name
  is per transaction and is created once.
- Stale data: Candidate construction, gating and deadlines read one transaction
  snapshot; deadlines, final epoch and clock checks keep their order.
- Reconciliation divergence: Compare the previous and new result sets (items,
  channels, coverage, qualifications, provenance, deadlines) through the unchanged
  retrieval, semantic, graph and answer platform tests.

## Integrations and Runtime Inputs

- Providers: PostgreSQL 17 with pgvector (repository-owned); no model call changes.
- Environment: Existing variables only.
- Secrets: None added; plans and counts recorded here contain no content.
- Failure handling: Unchanged timeouts, `recall_busy`, `recall_timeout` and
  semantic scope-size refusal (5,000 scoped representations, decided from the
  slim identity table before any provider request).

## Tests and Acceptance

- Automated: The existing `retrieval::`, `semantic::`, graph-retrieval, answer and
  memory platform tests pass unchanged; a new statement-counting test (100
  candidates, lexical and automatic recall, statement counts via the sqlx query
  tracing target); a plan test asserting the `recall_vector` GIN is used for a
  lexical query on a populated Brain; span-read unit/integration cases (missing,
  length mismatch, changed bytes); `cargo fmt`, Clippy, `./scripts/validate.sh`.
- Manual: Read-only `EXPLAIN` recorded in the closeout.
- Acceptance: Identical results to the pre-change run on the unchanged tests; EXPLAIN
  shows a GIN index/bitmap scan on `recall_vector` for a lexical query; statements
  per automatic recall with 100 candidates fall by at least 10×; the pack records
  measured before/after counts and any deviation from this specification.

## Closeout

- Planned: Items 1–3 above with the proof listed.
- Shipped: (1) Lexical window: migration 065 adds `recollect_window_*` functions
  (`SECURITY DEFINER`, membership test first, identifiers only) because the app
  role's row-level security makes `@@`/jsonb extraction non-leakproof and so
  ineligible for the GIN and literal indexes; they return the identifiers reachable
  through `recall_vector` GIN, hash-literal and partial coverage indexes. The source
  window uses `recollect_lexical_prefilter`, a superset relaxation of the parsed
  query, instead of an expression GIN index (the source vector concatenates two
  tables, so no single index exists); the exact `@@` recheck and `ts_rank_cd`
  (computed only for matched rows) are unchanged. `recollect_claim_status_eligible`
  is the single status predicate. (2) One candidate pass: `retrieval/pool.rs` builds a
  transaction-local `recall_pool` once; coverage, ranking, semantic admission/scoring
  and the brief/continuation read it. Semantic admission uses an identity-only table
  and drops it before the post-embedding transaction builds the full pool. (3) Set
  gating: `retrieval/gates.rs` loads scope validity, claim views
  (`memory::base_views`/`views`, now also the implementation behind `base_view`),
  evidence rows and availability, source/manifest currency, rules, conflicts
  (`memory_rules::matching_many`/`conflicts_many`), knowledge end, deadlines and
  raw-rule blocks per page of rows; chunk bytes use a bounded `artifacts::read_span`.
  Statements per recall with 100 candidates: 350/351 before, 26/27 after (about 13×,
  fixed per request rather than per candidate). Plans and index scans are recorded in
  the measurements below.
- Also shipped (follow-up on main's checkpoint f1b6bb3, migration renumbered 065):
  claim list paging now keeps reading candidate pages (at most 25) until the mode
  yields something, using the batched views, and `next_offset` advances past
  everything examined; the `kind` filter is a bound parameter; the fact-time raw-rule
  check was already batched with the other raw-rule checks. Window functions declare
  `ROWS 10`: main's new `source_versions` RLS policy made the planner scan the whole
  table (4.3 s for 20,000 rows) when it assumed 100 identifiers, and 13 ms with 10.
  Test: `claim_list_keeps_paging_past_pages_the_mode_excludes`.
- Not shipped: A literal NOT EXISTS over `assertion_rules` for conflicts (validity
  and scope overlap stay in `memory_rules::overlaps`, so candidates and their
  matching rules are fetched in sets and filtered there); an expression GIN index for
  the concatenated source vector; a single pass for semantic requests (the query
  embedding commits a transaction, so they build two); the graph channel's own
  entity-window SQL; a stored differential fixture for conflicts; a faster
  `recollect_memory_supported` (about 25–50 ms per revision for the app role, which
  still bounds claim-heavy recalls); installed-runtime deployment.
- New blockers: None for this slice. Fifteen platform tests that fail identically
  before and after this change (stale fixtures that use unreviewed proposals as
  usable support, plus capture/learning and MCP cases) need their own repair.
- Docs updated: hybrid-retrieval epic and index, active/archive indexes, this pack,
  measurements below.
- Validation: after merging main (`1e83b6b`), the same suites were re-run: the failing
  set is unchanged apart from main's own organization tests (two stack-overflow aborts in
  `models::organization_*`, not exercised by this slice; their remaining cases were not
  run); the three new tests in `retrieval::statements` pass. Earlier: `cargo fmt`; Clippy (no new warnings in changed files); unit test for
  span reads; `retrieval::statements` (statement counts and populated-table index
  proof) pass; `retrieval::` 9 pass/3 fail, `models::` 99 pass/4 fail, `memory::`,
  `review::`, `retention::`, `mcp::`, `capture`, `graph::` with only failures that
  also fail at the same line on the parent commit (listed in the mapping);
  `./scripts/validate.sh` passes.
- Version: N/A: no release policy.
- Commit: Committed on the worktree branch only (not pushed); see the branch tip.

## Measurements and plans (2026-10-19)

Observed: 2026-10-19
Confidence: observed-once

### Sources and Method

Local PostgreSQL 17 (pgvector image used by the repository compose stack) and the
platform test harness, run against disposable databases created by
`crates/server/tests/platform.rs`. The previous code was built from the checked-out
parent commit in a separate worktree and run with the same test file. Statements are
counted by a tracing layer on the `sqlx::query` target
(`crates/server/tests/platform/retrieval/statements.rs`); counts include
transaction control statements. Plans come from `EXPLAIN` of the window queries and
`pg_stat_user_indexes.idx_scan` deltas around a real HTTP recall. Nothing here was
run against the installed normal-data Brain, and no release or deployment occurred.

### Observations

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
  Migration 065 therefore resolves candidate identifiers in `SECURITY DEFINER`
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

### Translation and Limits

- The failing tests above are not caused by this slice, and they do not exercise
  claim eligibility through recall, so conflict and rule equivalence rests on the
  passing support, handover, autonomous and procedure suites plus code review; there
  is no stored differential fixture for `conflicts_many`.
- The windows cover the lexical-only request. Semantic scoring and the project brief
  need query-independent populations and use the full candidate pass (once per
  transaction; a semantic request has two transactions).
- Larger populations of claims remain bound by `recollect_memory_supported` per claim;
  that function is outside this slice.

### Follow-up

- Refresh stale platform fixtures to use reviewed or audited claims, then add a
  conflict/rule differential case.
- Measure on the installed Brain after the owner approves a migration 065 update.
- Consider a set-returning or cached form of `recollect_memory_supported`.
