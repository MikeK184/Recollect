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
- Migration `068_support_gate_definer.sql` recreates `recollect_memory_supported`
  as `LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp`. It
  returns false when `recollect_role(b)` is NULL, otherwise it evaluates the
  unchanged 043/046 predicate tree. Results are identical because every read
  policy on the tables the tree reads reduces to Brain membership (derived
  from `pg_policy` and enforced by a guard test); the actor GUC is session
  state, so `recollect_role` inside the function still sees the caller. The
  pattern is that of the migration 065 window functions. The migration owner
  bypasses row security, as it does for those functions.
- `recollect_memory_supported_many(b uuid, targets uuid[])` returns
  `(revision_id, supported)` per distinct target with the same membership
  check. It computes one recursive closure for all targets, evaluates each
  distinct revision once with the existing per-revision gates, then combines
  per target exactly as the scalar form: every closure member supported and
  acyclic, no non-root handover, every member digest-current.
- Call sites that test many revisions per statement use the batch function:
  `memory_rules::supported`, the claim list count and page, the recall and
  semantic candidate statements, session digest discovery/contributor checks,
  the graph descriptor roots, learning family matches, autonomous family
  matches and the migration 057 field input function. Single-revision sites keep
  the scalar function.
- Guard test: enumerates the tables the predicate tree reads and fails when any
  policy on them is not a permissive Brain-membership form, so a future
  restrictive policy cannot be silently bypassed by the definer.
- Equivalence test: on generated fixtures (rules, fences, digests, handovers,
  cycles, another Brain's revisions, a non-member actor) the scalar and batch
  results equal a reference copy of the previous body created in the
  disposable database. A timing test records app-role per-revision cost.
- Organization field paging: the statement that exceeds its bound is captured
  from the disposable database before any change. The planned repairs, each
  kept only if it measurably helps, are bulk qualification in the field view
  transaction like every other organization reader, a batch
  `recollect_field_inputs_current`, an index on
  `knowledge_field_inputs(brain_id,id)`, windowed known snapshots/manifests,
  and qualifying only the inputs behind the next page of references on detail
  pages (keyset, bounded). Outcomes are recorded here at closeout.

## Acceptance

- Automated: Policy guard and equivalence tests; `organization_fields` paging;
  the memory, claims, recall, digest and organization platform subsets compared
  with a baseline run on the same tree before the change; `cargo fmt`,
  `cargo clippy` as in CI and `./scripts/validate.sh`.
- Manual: N/A; timings are measured by the native test as the application role.
- Acceptance: Identical support results for members and non-members on every
  fixture, a measured per-revision cost reduction, and a passing field paging
  test without weakening any bound.
