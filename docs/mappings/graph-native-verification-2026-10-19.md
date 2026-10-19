# Graph native verification: query plans and proof receipts

Observed: 2026-10-19 (Europe/Vienna)
Confidence: observed-once on the local compose Neo4j 2026.08.1 Community (Cypher 25) and PostgreSQL 17 with a synthetic data set; the receipts below are not a deployment or installed-runtime claim

## Sources and Method

Authority is the [concurrent preparation contract](../contracts/graph-concurrent-preparation.md#native-verification-amendment--2026-10-19)
and its [execution pack](../roadmap/execution/active/graph-concurrent-preparation.md).
Plans were read with `PROFILE` through the Query API against the local compose
Neo4j, using one synthetic Brain (a UUID reserved for the measurement, removed
afterwards) holding 20 generations of 1,000 members and 5,000 relationships each
(101,094 `RECOLLECT_GRAPH_EDGE` relationships in the database). Indexes were
created and dropped on that same database, so the statements ran against the
actual installed graph schema. Only aggregate relationship and generation counts of
the installed graph were read; no installed Brain data was written. The
Query API call counts come from a counting proxy in front of the real Neo4j in
the platform tests named below.

## Observations

Database hits per statement, before and after the generation indexes:

| Statement | Without index | With index |
| --- | --- | --- |
| Per-read seal, two generations | relationship type scan, 202,188 rows; 414,388 hits | `DirectedRelationshipIndexSeek (generation)`, 10,000 rows; 20,012 hits |
| `verify_counts` relationships | type scan; 277,346 hits | index seek; 10,001 hits |
| `verify_counts` members | `Expand(All)` from the generation, 1,004 hits | `getDegree` projection (`COUNT{}`), 3 hits |
| Cleanup and reset, one 500-relationship batch | type scan to the batch; 18,333 hits | index seek; 1,002 hits |
| `verify_edges`, 500 ids | label scan plus type scan; 460,789 hits | `(generation)` index only: 28,001 hits (whole generation per batch); `(generation, id)` composite: 5,500 hits |

A composite `(generation, id)` index is used only when the statement names both
properties and cannot serve the generation-only counts and deletes, so the adapter
creates both. Counting 100,000 relationships over 20 generations in one seal
request took 31 to 65 ms.

Query API requests per read (the fixture holds 7 entities and 6 relationships):

| Read | Before | After |
| --- | --- | --- |
| Graph view, cold | 1 + ceil(nodes/500) + ceil(edges/500): 3 here, up to 51 at the read limits | 1 |
| Graph view, repeated within 20 s | 1 (generation existence only; damage went unnoticed) | 1 (full seal) |
| Path with two returned relationships | the view cost plus 1 search | 3: seal, search, returned relationships |

The "before" counts follow from the removed code path, not from a proxy at the old
revision.

Platform receipts (single-threaded, UUID-owned PostgreSQL databases, the local
Neo4j, `RUST_MIN_STACK=8388608`; subset filter `graph:: autonomous:: semantic::
maintenance`):

| Run | Result | Failures |
| --- | --- | --- |
| Before: main at 8228549, 89 tests, 887 s (with a 16 MiB stack) | 87 passed, 2 failed | `recovery_analytics_ownership_is_mirrored_before_native_work_and_retained_for_restore` (needs the owned SFTP fixture variable), `graph_lost_lease_cancellation_and_writer_revocation_cannot_publish` (assertion at `graph_recovery.rs:523`) |
| After: 92 tests, 723 s | 90 passed, 2 failed | The same two, with identical messages |

A wider regression subset that touches graph reads, analytics invalidation or
the maintenance loops (`brain_admission`, `retrieval`, `models::answers`,
`mcp_memory`, `organization*`, `knowledge_mapping`, `production*`: 85 tests) and the
migration upgrade case ran after the change: 80 passed and 5 failed, and the same 5
(`organization_native_browser_uses_automatic_worker_and_unmocked_scoped_api`,
`organization_overlays_source_snapshot_survives_vector_replacement_and_no_recipe`
and three `retrieval` cases) fail identically on the baseline binary. One batch
process ended without a result line (no panic output; most likely killed under
memory pressure while other builds ran), so the 39 tests it had not reported were
rerun by name.

The three added tests pass: two actual-Neo4j cases in `graph_seal.rs` and the
held-admission-lock case in `maintenance_budget.rs` for the per-Brain maintenance
budgets. That case holds the exact fair-admission lock of one of two Brains: the
semantic pass still finished in under 15 s, queued only the free Brain's entries
and batches and left no semantic profile, entry or batch row for the held Brain;
the autonomous pass queued one learning run for the free Brain only and recorded no
scan cursor for the held one; after the lock was released both passes served the
held Brain without duplicating the free Brain's work. In `graph_seal.rs` a
counting proxy in front of the real Neo4j showed one Query API request per warm
view (repeated twice), three per path, immediate `graph_import_mismatch` on the
very next view and path after one relationship was deleted out of band, success
(more than one request) after an unselected entity was erased, and failure again
once a selected entity was erased. A due generation was restamped with no change
to the analytics epoch; a sealing token changed while verification was in flight
was not stamped (the case fails if that guard is removed); and for both a deep
check and a failed read, exactly one replacement was queued across four passes and
built, with the damaged generation failing reads until then.

## Translation and Limits

The seal proves token, closure and cardinality. A substitution that keeps both
counts (one relationship removed and another created with the same generation)
changes neither, so it is seen only by the exact check of the entities and
relationships a read selects or a traversal returns, or by the 24-hour deep check.
A view returns canonical data from PostgreSQL, so such a swap cannot add
non-canonical content to a response. Generations published before migration 048
have no recorded sealing token; they use the exact fallback on every read until
rebuilt. The exact edge check is linear per batch with the composite index, but a
deep check of a generation near the 250,000-relationship limit still issues about
700 requests inside its Brain's 30-second budget; no such generation exists in the
measured installation. The two indexes are created in the shared local Neo4j by
the tests and by any adapter schema call; they are additive.

## Follow-up

Normal-installation update and a post-update populated read timing remain open in
the pack; this record does not replace the October 18 normal-data timings.
