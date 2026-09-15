# Graph projection and traversal validation — 2026-09-15

## Scope and method

Confidence: verified for the executed local checks below. The working tree is
uncommitted and no release/deployment is claimed. The
[accepted contract](../contracts/graph-projection-and-traversal.md) owns the
behavior; [interface evidence](graph-interfaces-2026-09-15.md) and the
[Cognee/Atlas review](graph-reuse-review-2026-09-15.md) record external sources
and the reusable mechanisms. Reference checkouts were read only.

Tests used UUID-owned PostgreSQL databases and actual loopback Neo4j Query API
calls. A controlled proxy delayed or replaced individual responses while normal
imports/traversals still used Neo4j. Successful fixtures remove only their own
Brain graph labels; failed fixture databases remain available for diagnosis.
No customer files or paid model requests were needed for these checks.

## Executed acceptance

| Behavior | Evidence |
| --- | --- |
| Distinct duplicate upstream IDs, unresolved/ambiguous/unsupported targets, cycles, direction/hop bounds, rejection and an eligible longer path | `graph_structural_publication_prefilter_rejection_and_rebuild` |
| Rebuild, shared entities, generation cleanup, missing physical edge distinguished from no path, zero-hop explicit endpoint | Same structural proof plus `graph_partial_import_retries_frozen_descriptors_and_fences_old_generation` |
| Current knowledge supports, typed handover contributions, strict review/operational modes, revalidation and ancestry boundaries | `graph_knowledge_preserves_typed_contributions_review_and_current_revisions` |
| One rejected fragment withholds its complete source vertex; unrelated claim survives; direct source-chunk RLS isolation | `graph_source_vertex_withholds_all_fragments_when_one_assertion_is_rejected` |
| Partial import is unpublished; malformed acknowledgement retries the same descriptors; delayed old-generation writes remain fenced | Partial-import proof against the real store |
| Lost lease, cancellation/retry and revoked writer cannot publish over current work | `graph_lost_lease_cancellation_and_writer_revocation_cannot_publish` |
| Transient versus permanent Neo4j errors, malformed fields/counts and HTTP timeout | Backend-error proof and `graph_http_timeout_cannot_publish_a_delayed_import` |
| Erasure remains canonical during outage, physical completion is pending, a delayed import cannot restore erased keys, startup replays restored identities | `graph_erasure_blocks_late_import_and_replays_physical_cleanup_after_outage` |
| Expiry during delayed publication rolls back readiness; heartbeat continues while its renewal waits | `graph_publication_crossing_retention_deadline_rolls_back_and_renews_without_deadlock` and the existing semantic publication regression |
| Expiry after a path query is withheld; maintenance rebuilds expired knowledge without requiring an epoch edit | `graph_path_checks_expiry_after_query_and_maintenance_rebuilds_without_epoch_edit` |
| Exact old production manifest, explicit old snapshot, native immutable operation, revoked grants and direct generation/fact RLS | `graph_exact_manifests_native_operations_and_rls_keep_brain_boundaries` |
| 5,001-node read refuses before path work; 100,001-node generation fails without Neo4j publication | `graph_large_inputs_and_reads_refuse_explicitly_without_truncation` |
| Browser processing, typed path, inert retained evidence, no-path/error states, scope changes, no automatic replay and mobile layout | `web/tests/graph.spec.ts`; real services, latest pass 6.6 seconds; desktop/mobile screenshots visually inspected |

The complete platform command passed **73 tests in 174.30 seconds**, including
existing publication, review, retention, automatic learning, handover, capture
and semantic behavior. Its separately hosted OIDC test was excluded by the
existing script. `cargo test --workspace` passed ten ordinary tests; ignored
service tests were run by the platform/browser commands. Clippy with warnings
denied passed. Governance validation passed all 32 checker tests.

Artifacts are in `.cache/graph-proof/`: `platform.log`, `workspace.log`,
`clippy.log`, `ui.log`, `authority-validation.log`, focused failure/recovery logs
and browser screenshots. Initial fixture/stack/performance failures are described
in the linked mappings; they were repaired rather than excluded from acceptance.

## Normal runtime boundary

Before migration 018, read-only desktop/mobile recall verified all seven normal
Brains and the retained SWEG snapshot with zero additional model requests or
customer file reads (`preservation-before.json`, verified 16:20:21 UTC). A local
PostgreSQL dump was retained at `normal-before018.dump` before restart.

The updated normal app applied migration 018 and started at 16:33:08 UTC. At
16:41:22 UTC the actual API and desktop/mobile browser selected the retained SWEG
snapshot `260c6505-f1b8-40ad-bb2a-c7db5cd48a1f` and exact manifest
`6f558274-57c4-4007-aefb-10ad028221ca`. Generation
`fcab9fb0-c127-4137-a910-90383fe9a64f` had 99 eligible entities and 183 edges.
The existing `var.ssh_public_key` fact was selectable as an exact zero-hop path;
the UI retained partial reasons for truncated fragments and unresolved targets.
Both viewport screenshots were visually inspected, with no mobile overflow or
browser errors. Searchable multi-select inputs fixed the normal repository
selector interaction; the synthetic browser regression passed again afterward.

At 16:41:55 UTC, the independent read-only recall preservation check passed after
the migration: all seven Brains, exact SWEG manifest/fact, proposed-claim strict
exclusion, autonomous revision, supersession history and retained capture survived.
Model totals remained 6, 4, 3, 20, 0, 0 and 0 for their respective existing Brains;
both checks made zero new model calls and read zero customer source files.
Evidence is `runtime.json`, `runtime-{desktop,mobile}.png`,
`preservation-after.json` and `preservation-after.log` under the same cache folder.
The [runbook](../runbooks/graph-projection-and-traversal.md) describes the delivered
workflow and recovery limits.

Cross-repository links, GDS analytics, full graph rendering and retrieval fusion
remain owned by their successor slices.
