# Semantic recall over large scopes: measurements

Observed: 2026-10-19
Confidence: observed-once

Evidence for the [semantic recall contract](../contracts/retrieval-semantic.md)
bounds. Synthetic vectors on one shared development machine; not a model-quality,
production-latency or cold-cache claim.

## Sources and Method

- Code: the `work/security` worktree on the commit that precedes the
  `retrieval-semantic-large-scopes` slice, with that slice's changes. Database:
  PostgreSQL 17.10 with pgvector 0.8.6 in the local compose container (aarch64,
  2 CPUs, 1 GiB memory limit, `shared_buffers` 128 MB, `work_mem` 4 MB) and
  disposable databases. pgvector `binary_quantize` and the Hamming operator `<~>`
  checked against the
  [pgvector documentation](https://github.com/pgvector/pgvector#binary-quantization)
  (Context7, 2026-10-19): `binary_quantize(vector)` returns a bit string (component
  greater than 0 is 1) and the documented re-rank pattern is the one used here.
- Fixture: repository facts supply distinct identities. Three repositories of
  5,000, 15,000 and 30,000 facts give scopes of 5,000, 20,000 and 50,000 entries at
  8, 384, 1,536 and 3,072 dimensions. Background vectors are independent uniform
  noise (seeded `random()`); 150 planted neighbours at cosines from 0.9 to 0.35 of
  one query sit in the first repository. Two queries: the planted query, and an
  independent random query (the adversarial case: no neighbour is meaningfully
  closer than the background). Ground truth is the exact statement over the same
  scope.
- Timing: `EXPLAIN (ANALYZE, BUFFERS)` of the production statement as
  `recollect_app` under JIT off, three runs each; best and median of three, warm
  cache (cold cache not measured). The machine also ran other builds and tests,
  and the same statement varied up to four times between runs.
- Command: `cargo test -p recollect-server --test platform -- --ignored
  --test-threads=1 --nocapture semantic_scale_` (four tests, one per dimension).

## Observations

Cause. At 50,000 facts the candidate pool took 5.9 s of a 6.1 s statement because
`recollect_retention_deadline(brain,class,captured)` was evaluated for every
candidate row at about 160 µs (a policy lookup under row-level security); the day
counts are now resolved once per class. Pool build before and after, 1,536
dimensions: 5,000 rows 704 to 31 ms, 20,000 rows 4,997 to 94 ms, 50,000 rows
5,879 to 334 ms. Computing the distance once instead of twice moved the exact
statement at 5,000 rows and 3,072 dimensions from 105-163 ms to 97-178 ms (five
runs each, one cold run excluded): no measurable gain. A full `ANALYZE` of the
pool cost 0.5-1.0 s at 50,000 rows without changing any plan; the column-limited
form costs under 20 ms.

recall@100 of the shortlist against exact, planted query / independent-noise query
(M is the shortlist size; production uses M=4,000):

| dims | rows | M=1,000 | M=2,000 | M=4,000 | M=8,000 |
| --- | --- | --- | --- | --- | --- |
| 3,072 | 5,000 | 1.00 / 0.81 | 1.00 / 0.96 | 1.00 / 1.00 | 1.00 / 1.00 |
| 3,072 | 20,000 | 1.00 / 0.68 | 1.00 / 0.81 | 1.00 / 0.89 | 1.00 / 0.99 |
| 3,072 | 50,000 | 1.00 / 0.56 | 1.00 / 0.78 | 1.00 / 0.87 | 1.00 / 0.94 |
| 1,536 | 5,000 | 1.00 / 0.77 | 1.00 / 0.94 | 1.00 / 1.00 | 1.00 / 1.00 |
| 1,536 | 20,000 | 1.00 / 0.63 | 1.00 / 0.75 | 1.00 / 0.95 | 1.00 / 1.00 |
| 1,536 | 50,000 | 1.00 / 0.50 | 1.00 / 0.71 | 1.00 / 0.83 | 1.00 / 0.94 |
| 384 | 50,000 | 1.00 / 0.46 | 1.00 / 0.72 | 1.00 / 0.88 | 1.00 / 0.94 |
| 8 | 50,000 | 0.72 / 0.37 | 0.97 / 0.58 | 0.98 / 0.68 | 0.99 / 0.89 |

Statement time as the application role, best / median of three, milliseconds. The
exact column over the bound shows what the tier would cost (production ranks those
scopes by the shortlist, except at 384 dimensions and below, where exact is used):

| dims | rows | exact statement | shortlist statement | whole request |
| --- | --- | --- | --- | --- |
| 3,072 | 5,000 | 172 / 174 | 58 / 62 | 373 (exact) |
| 3,072 | 20,000 | 794 / 1,108 | 218 / 373 | 926 (shortlist) |
| 3,072 | 50,000 | 2,334 / 2,488 | 688 / 828 | 1,640 (shortlist) |
| 1,536 | 5,000 | 130 / 143 | 95 / 96 | 428 (exact) |
| 1,536 | 20,000 | 599 / 792 | 203 / 208 | 909 (shortlist) |
| 1,536 | 50,000 | 1,992 / 2,371 | 415 / 494 | 1,235 (shortlist) |
| 384 | 5,000 | 45 / 89 | 85 / 88 | 363 (exact) |
| 384 | 20,000 | 290 / 361 | 132 / 184 | 794 (exact) |
| 384 | 50,000 | 839 / 920 | 234 / 271 | 1,709 (exact) |
| 8 | 5,000 | 19 / 21 | 31 / 32 | 344 (exact) |
| 8 | 20,000 | 114 / 119 | 95 / 120 | 612 (exact) |
| 8 | 50,000 | 382 / 425 | 258 / 378 | 1,229 (exact) |

The whole request includes admission, the query embedding call to a local
fixture, the full candidate pass and ranking.

## Translation and Limits

- Planted neighbourhoods survive the shortlist at 384 dimensions and above
  (recall 1.00 at every size). Independent noise is the adversarial case, and its
  recall falls with scope size and rises with M: at 50,000 rows and M=4,000 it is
  0.83-0.88. Real embeddings are neither independent noise nor planted, so these
  numbers bound nothing about a model; measure on a real corpus before relying on
  a recall figure.
- At 8 dimensions sign bits carry too little information (planted recall 0.72 at
  M=1,000, noise 0.68 at M=4,000), and exact ranking over 50,000 rows measured
  0.4-0.9 s: profiles of at most 384 dimensions rank exactly over any scope.
  Exact over 50,000 rows at 1,536 and 3,072 dimensions measured 2.0-2.5 s, over
  the statement bound, so those profiles keep the 5,000-row exact bound. Exact at
  20,000 rows (0.6-1.1 s warm) was not adopted: a cold cache reads 240 MB of
  out-of-line vectors at 3,072 dimensions and was not measured.
- Not established: cold-cache latency, four concurrent recalls against the two-CPU
  container, real-corpus recall, and any vector-index alternative (none was built).

## Follow-up

Measure recall@100 on a real embedded corpus (the declared engineering corpus of
the semantic contract) before quoting a recall figure for users; consider a larger
M or a third page only if that measurement shows misses the paging cannot recover.
