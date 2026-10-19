# Support gate cost and organization field paging

Status: in-progress
Owning epic: `docs/roadmap/epics/hybrid-retrieval.md`
Work type: product
Implementation: authorized by the repository owner on 2026-10-19

## Summary

- Goal: The canonical support predicate costs well under a millisecond per
  revision for the application role (it measured 25-50 ms), so recall, claim
  lists, digests, graph descriptors and field organization stop spending their
  budgets on row-level security bookkeeping, and the organization field view
  pages its 85-input fixture inside its bounded window.
- Non-goals: Change what is supported, any recall ranking or gate, any
  authorization rule, any read policy, or the Brain membership model.
- Delivery shape: One migration, set-oriented call-site switches, field-view
  paging repair, and native equivalence, policy-guard and timing tests.

## Governing Sources

- [Source support before usable memory](../../../contracts/memory-source-support-verification.md)
- [Automatic knowledge organization](../../../contracts/automatic-knowledge-organization.md)
- [Canonical exact and lexical recall](../../../contracts/retrieval-exact-and-lexical.md)
- [Owning epic](../../epics/hybrid-retrieval.md)

## Scope

- In scope: A SECURITY DEFINER `recollect_memory_supported(b, target)` with an
  up-front Brain membership check; a batch `recollect_memory_supported_many`;
  switching set-oriented callers to it; a batch field-input currency function,
  a keyset index and bulk qualification for the organization field view.
- Out of scope: Rewriting the support predicate tree, caching verdicts, new
  read policies, restrictive policies on any table the predicate reads, and
  changing the field view's wire format.
- Blockers: None.

## Decisions

- Diagnosis: Every table the predicate reads has a per-row read policy
  `recollect_role(brain_id) IS NOT NULL`, and `recollect_role` and
  `recollect_effective_role` are SECURITY DEFINER SQL functions with SET
  clauses, so a policy evaluation costs a function-context switch per visited
  row. The same predicate costs about 0.8 ms per revision as the superuser, so
  the cost is policy evaluation, not plans or indexes.
- Migration `068_support_gate_definer.sql` splits the scalar function by whether
  row-level security applies to the caller. `recollect_memory_supported_core` is
  the unchanged 043/046 predicate with the caller's rights;
  `recollect_memory_supported_member` is
  `LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp`, returns
  false when `recollect_role(b)` is NULL and otherwise evaluates the core; and
  `recollect_memory_supported` (same name, owner and grants) calls the member
  form when `row_security_active('claim_revisions')` and the core otherwise.
  Results are identical because every read policy on the tables the predicate
  reads reduces to Brain membership (derived from `pg_policy` and enforced by a
  guard test); the actor GUC is session state, so `recollect_role` inside the
  function still sees the caller. The pattern is that of the migration 065
  window functions, and the migration owner bypasses row security as it does for
  those. The first design (a plain definer function with the membership test)
  broke `automatic_digest_preserves_completed_responses_and_retained_inputs`:
  migration 039's owner-rights `recollect_privacy_closure` calls the predicate
  without an actor, got false, and cancelled a valid digest job. Owner-rights
  callers therefore keep their unfiltered evaluation, which is also what an
  invoker-rights function gave a caller that row security does not apply to.
- `recollect_memory_supported_many(b uuid, targets uuid[])` returns
  `(revision_id, supported)` per distinct target with the same membership
  check (for actor contexts; a caller without a Brain role gets false, the safe
  answer). It computes one recursive closure for all targets, evaluates each
  distinct revision once with the existing per-revision gates, then combines
  per target exactly as the scalar form: every closure member supported and
  acyclic, no non-root handover, every member digest-current.
- Call sites that test many revisions per statement ask the batch function for
  the revisions that passed their cheap filters: the recall candidate chain,
  the claim list count and page, semantic discovery, session digest discovery
  and contributor check, graph descriptor roots, learning and autonomous family
  matches, and `memory_rules::supported`. Single-revision sites keep the scalar
  function. Migration 057's field input function is not rewritten: after this
  change its support call is the fast definer form and the whole function costs
  about 1 ms per input (see below).
- Guard test: enumerates the tables the predicate reads from the function
  bodies and fails when any policy on them is restrictive, narrower than Brain
  membership, or absent, so a future policy cannot be silently bypassed by the
  definer. It also proves it can fail by adding such policies in its disposable
  database.
- Equivalence test: on generated fixtures (human-reviewed claims, an unreviewed
  one, a rejected one, a rule-blocked one, a fenced one, handovers including a
  non-root handover dependency and an unsupported current head, exact
  contribution cycles, expiring sources, a second Brain's revisions, a
  non-member actor) the scalar and batch results equal a reference copy of the
  previous body created in the disposable database and called as the
  application role. Existing support scenarios (digests, rules, cycles,
  backups) assert the same equivalence at each state they inspect.
- Organization field paging: the failing statement is the recall candidate
  statement inside `retrieval::graph::qualify`, not the 750 ms publication step.
  It hit the 1,500 ms prepare limit (SQLSTATE 57014) in 2 of 4 runs here and
  every baseline run, taking 1.1-1.3 s for 85 requested facts against 47-75 ms
  in runs that passed. `EXPLAIN ANALYZE` of a slow run shows the planner joining
  each windowed fact to `repository_snapshots` by scanning the Brain's snapshots
  (1,187 rows removed by the join filter for 25 facts) and evaluating the
  row-level-security function on every scanned row; the plan varies from run to
  run. A first repair resolving each fact's snapshot by primary-key lookup was
  rejected: it removed the flip but evaluated row-level security once per fact
  and made `graph_combined_union_limits` (5,200 facts) fail with
  `database_unavailable`. The repository-fact branch now joins a materialized
  `fact_snapshots` CTE that applies the snapshot-only gates (time, retention
  state, scope) once per snapshot the windowed facts belong to; recall and graph
  windows narrow it through a new window hook, and the graph qualify path also
  windows `known_snapshots`/`known_manifests` through the existing hooks. The
  statement measured 1,006 ms in the plan that failed and 110 ms now, and the
  paging test and the union-limit test passed 6 of 6 and 1 of 1 consecutive
  runs under load.
- Kept: the keyset index `knowledge_field_inputs_page(brain_id,id)`: 60,000
  inputs in 20 Brains read 4,500 rows in 1.7 ms before and 251 rows in 0.27 ms
  after.
- Not kept, because nothing measurable improved: `recollect.bulk_qualification`
  in the field view (three runs each, 192-420 ms against 245-500 ms total on a
  loaded machine, which is noise; that fixture reads no policy the setting
  rewrites); a batch field-input currency function and narrowing detail pages
  to the next page of references (the currency step costs about 1 ms per input
  and the 85-input page about 250 ms, far inside the 1,500 ms bound once the
  candidate statement is stable).

## Acceptance

- Automated: Policy guard and equivalence tests; `organization_fields` paging;
  the memory, claims, recall, digest and organization platform subsets compared
  with a baseline run on the same tree before the change; `cargo fmt`,
  `cargo clippy` as in CI and `./scripts/validate.sh`.
- Manual: N/A; timings are measured by the native test as the application role.
- Acceptance: Identical support results for members and non-members on every
  fixture, a measured per-revision cost reduction, and a passing field paging
  test without weakening any bound.
