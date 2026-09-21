# Repository and Foundation Bootstrap

Status: shipped
Owning epic: `docs/roadmap/epics/repository-governance.md`
Work type: governance

## Summary

- Goal: Install Recollect's strict documentation lifecycle and Context7 setup,
  then document the user's initial Cognee extension design.
- Non-goals: Application implementation, custom skills, or deployment.
- Delivery shape: Local uncommitted documentation, configuration, and validation tooling.

## Governing Sources

- [ADR 0001](../../../adr/0001-repository-governance.md)
- [Repository governance contract](../../../contracts/repository-governance.md)
- [Owning epic](../../epics/repository-governance.md)

The user's 2026-09-13 follow-up extends this slice to author all three foundation
documents, inspect Cognee's existing stack and integrations, and recommend an
upstream synchronization and development-tooling approach. This is documentation
authority; it does not request application implementation or remote changes.

## Scope

- In scope: New Git root, documentation buckets/templates, foundation documents
  from the supplied design, six agent roles, Context7, lifecycle validator and
  adversarial fixtures, dated upstream evidence, and development/upstream guidance.
- Out of scope: Cognee edits, application code, custom skills, other MCPs,
  remote creation, automatic releases/commits, publishing, and deployment.
- Blockers: None for this documentation slice; detailed product contracts remain future work.

## Surface and Interface Changes

- Interfaces: Root validation command and documented Markdown lifecycle fields.
- Storage: N/A: no application storage or migrations.
- Ownership: Recollect owns its docs, scripts, and .codex files; Cognee stays separate.

## Data and Authority

- Inputs: Historical reference lifecycle, current reference Context7 stanza,
  user-approved plan, local Codex guidance, and current checkout evidence.
- Authority: Accepted bootstrap ADR/contract and explicit user intent.
- Blind spots: Detailed product contracts and deployment storage choices remain
  future work. Upstream capabilities are inspected statically, not runtime-proven.

## States and Edge Cases

- Loading: Validator reads only governed files; no network or Cognee traversal.
- Empty: Empty roadmap indexes are valid; pending foundations are reported.
- Error: Invalid documents/configuration exit nonzero with a file-specific reason.
- Blocked: Record blockers in active packs; prohibit unresolved implementation authority.
- No-access: Context7 failures are reported separately from local configuration validity.
- Duplicate or replay: Duplicate slice IDs, packs, and index rows fail validation.
- Stale data: Historical reference records supply process only, not current product rules.
- Reconciliation divergence: Epic, pack, and index mismatches fail validation.

## Integrations and Runtime Inputs

- Providers: Context7 via npx; no other MCP integrations or product providers.
- Environment: CONTEXT7_API_KEY forwarded from the existing environment.
- Secrets: No credential values copied, logged, or stored in tracked artifacts.
- Failure handling: Offline validation remains independent of network; bound the
  separate Context7 smoke check and report actual connectivity or failure.

## Tests and Acceptance

- Automated: Validate repository structure/config and valid plus malformed
  temporary fixture trees for ownership, IDs, statuses, sources, and indexes.
- Manual: Verify Codex discovery, initialize/list/call Context7 read-only, and
  compare Cognee Git state plus tracked-file hash before and after.
- Acceptance: Validator and fixture checks pass; the supplied product design is
  captured in all three foundations with stock/extension boundaries; supported
  config is discoverable and connectivity verified or truthfully reported;
  Cognee is unchanged and no unrelated reference-product rules were imported.

## Closeout

- Planned: Documentation lifecycle, agent roles, Context7, validation tools,
  supplied product foundations, upstream investigation, and tooling assessment.
- Shipped: New Recollect Git repository, strict epic/pack lifecycle and templates,
  semantic governance validator, six role registrations, verified Context7,
  three accepted initial foundation documents, dated source evidence, and
  development-tooling/upstream-sync guidance.
- Not shipped: Application implementation, detailed runtime contracts, custom
  skills, additional MCPs, fork/remote/branch provisioning, CI hosting, deployment.
- New blockers: None for this slice. Later application work needs its scoped
  contracts; fork publication needs the selected writable repository destinations.
- Docs updated: Root instructions/README, docs lifecycle and templates, foundation
  baseline, bootstrap authority, epic/indexes, runbooks, and upstream evidence.
- Validation: Governance lint and 32 fixture tests passed via scripts/validate.sh.
  Codex discovered Context7; stdio initialization, tool listing, FastMCP library
  resolution, and documentation query passed against Context7 4.1.0. Cognee's
  branch/HEAD/remotes stayed unchanged and its worktree remained clean; all
  3,593 tracked files retained the same combined content/mode SHA256
  71e01ebae75f23dab36c1db3ae4f3cadbfc2cc82e4cf4bdd34a9988c8425ce1c.
- Version: N/A: no release policy.
- Commit: uncommitted.
