# Supersession edges are not support or deadline dependencies

Status: in-progress
Owning epic: `docs/roadmap/epics/memory-lifecycle.md`
Work type: product
Implementation: authorized by the repository owner on 2026-10-19

## Summary

- Goal: A claim revision that replaces or retires an earlier revision of the same
  claim stays supported, and keeps its own deadline, after the replaced
  revision's raw source expires. Today the support and deadline closure follows
  the replacement-to-predecessor edge, so the replacement inherits the old raw
  session's expiry and fails publication with `reconciliation_input_changed`.
- Non-goals: Change erasure closure, the support audit's exact-lineage
  prerequisites, handover/digest contribution semantics, review rules, source or
  manifest gates, or the cross-claim offered inputs of a reconciliation run.

## Governing Sources

[Source support before usable memory](../../../contracts/memory-source-support-verification.md),
[retention and erasure](../../../contracts/memory-retention-and-erasure.md),
[capture reconciliation](../../../contracts/memory-capture-reconciliation.md),
[cross-session reconciliation](../../../contracts/memory-cross-session-reconciliation.md),
[epic](../../epics/memory-lifecycle.md). The `input_deadline` comment in
`crates/server/src/autonomous.rs` already states the intent: historical reasoning
edges are erasure dependencies, not a permanent TTL dependency on every
superseded source.

## Scope

- In scope: migration `071_supersession_support_boundary.sql`; the support and
  deadline functions and the batch function stay consistent; the equivalence
  tests and the contract statements.
- Out of scope: Any change to `claim_contributions` rows or to who writes them.
- Blockers: None. Open follow-up decision (reported, not decided here): the
  other reconciliation inputs offered to a run (different claims of the same
  session) are also linked to every revision the run publishes and remain
  support dependencies.

## Decisions

- Edge kinds: `claim_contributions(revision_id,input_revision_id)` has no kind
  column and only two writers. `procedures::append` writes handover (including
  digest) contributions and rejects an input of the same claim or of kind
  handover. `autonomous::basis` writes every offered reconciliation input to each
  published replacement, retirement and new claim. A supersession edge is exactly
  an edge whose input revision has the same `claim_id` as its parent; only
  `basis` can produce one. Handover contributions, cross-claim offered inputs,
  supports, decisions, rules, manifests and digests are unchanged dependencies.
- Storage: migration 071 (070 is reserved by another branch; the runner compares
  applied names positionally with the compiled list, so a numbering gap is
  tolerated, but 070 must be inserted before 071 on any database that has not
  applied 071). It adds `recollect_memory_support_dependencies(b,target)`: the
  exact-contribution and current-head closure of `recollect_memory_dependencies`
  without same-claim edges (neither the exact edge nor the predecessor's current
  head). It redefines `recollect_pre_digest_supported`,
  `recollect_memory_supported_core`, `recollect_memory_supported_many` and
  `recollect_memory_deadline` over it. `recollect_memory_dependencies`,
  `recollect_memory_exact_dependencies`, `recollect_memory_exact_acyclic`,
  `recollect_support_audit_target` and every privacy closure keep the full graph,
  so erasure of a predecessor's source still erases the replacement where the
  retention contract requires, and cycle detection and audit prerequisites are
  unchanged. A cycle through a supersession edge still fails the root's exact
  acyclic gate.
- Interfaces: None; `input_deadline` reads the offered revisions' own supports
  and does not follow contribution edges, so it needs no change.
- Failure handling: Unchanged; an unsupported genuine dependency still makes the
  dependant unsupported and bounds its deadline.

## Acceptance

- Automated: the capture publication case passes; `support_gate.rs` reference
  encodes the new semantics for this edge kind only and proves scalar, batch,
  owner-rights and non-member equivalence; new cases cover (a) a replacement
  whose predecessor's source expired stays supported with its own deadline,
  (b) erasing the predecessor's source still erases the replacement through the
  privacy closure, (c) an expired genuine handover contribution still makes the
  handover unsupported; support, memory, retention, retrieval, review, digest,
  learning and capture platform subsets are no worse than the baseline measured
  on the same revision; format, Clippy and `./scripts/validate.sh`.
- Manual: N/A; database behavior is covered by the disposable-database tests.
- Acceptance: The contracts state the rule; the closeout summary names the
  commands and counts that were run.
