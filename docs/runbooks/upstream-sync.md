# Reference Checkout and Component Update Guidance

Revised: 2026-09-13. This is guidance for future dependency/reference updates,
not a report that an application upgrade was executed. The former proposal to
fork Cognee and cognee-integrations as mandatory product foundations is superseded
by the [independent Rust stack](../foundation/techstack.md).

## Repository ownership

Recollect owns its application and documentation in this repository. The ignored
cognee/ and agent-memory-atlas/ directories are independent reference checkouts.
They are not submodules, required runtime services or destinations for Recollect
product code. Preserve their histories, remotes and unrelated work.

Reference research does not authorize changing external services or rewriting
upstream history. Pin source references in dated mappings. Recollect-owned
implementations and selected dependency versions must remain reviewable in this
repository rather than hidden inside an ignored reference checkout.

## Reviewing a reference update

1. Inspect the exact checkout's status, origin and revision; preserve local work.
2. When an update is requested, fetch and inspect the intended upstream revision.
   Avoid an unqualified pull that silently chooses a new research baseline.
3. Record the inspected revision and changes affecting relevant claims. Existing
   mappings retain their original pins and dates; add a new assessment when needed.
4. Reconcile affected foundation/contract assumptions only when evidence or user
   decisions warrant it. A reference upgrade does not automatically change the
   selected product stack.

## Updating an implemented dependency

Once application dependencies exist, use the owning implementation slice:

1. Read current official interfaces, compatibility notes, licenses and migrations.
2. Pin the selected compatible Rust/frontend/container/extractor versions and
   preserve required license notices.
3. Run focused product proof, including scope, correction/replay and interface
   compatibility where affected. Use disposable data and explicit targets.
4. Specify data recovery for migrations; switching a binary or Git revision alone
   may not restore the previous storage format.
5. Reconcile evidence, contracts, runbooks and delivery records using actual
   validation. Commit, push, release and deployment follow the user's authorization.

If a concrete missing upstream extension point later requires a fork, document
that component boundary and its synchronization strategy at that time. No
mandatory Cognee fork, plugin fork or submodule conversion is selected today.
