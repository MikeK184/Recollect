# Repository Governance

Status: complete

## Purpose

Establish Recollect's strict documentation lifecycle, repo-local developer
configuration, and the initial Cognee extension baseline supplied by the user.

## Governing Sources

- [ADR 0001](../../adr/0001-repository-governance.md)
- [Repository governance contract](../../contracts/repository-governance.md)

## Dependencies and Boundaries

The user initially approved the governance bootstrap, then supplied the product
design and requested foundation documents plus upstream/tooling guidance.
Keep the separate Cognee checkout untouched. Product implementation, commits,
publishing, and deployment were deferred at bootstrap and later authorized by
separate assignments. The current 2026-09-26 instruction selects Apache-2.0 and
a simpler README while keeping GitHub private. Public visibility, repository
renaming and external deployment remain deferred. The later documentation-skills
assignment belongs to the Developer Tooling epic; it lifts the bootstrap's
deferral for the two requested documentation skills.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `repository-governance-bootstrap` | shipped | adr-backed, contract-backed | pack | Deliver lifecycle checks, roles, Context7, supplied product foundations, and upstream/tooling guidance |
| `foundation-access-and-workspace-clarifications` | shipped | contract-backed | small-fix: bounded documentation clarification and fixture-isolation repair; no runtime change | Clarify dataset ownership, SSO group mapping, managed-operation enforcement, and workspace/developer configuration boundaries |
| `repository-license-and-readme` | shipped | contract-backed | small-fix: established user license choice, README and package metadata; no runtime change | Apply Apache-2.0 consistently, simplify the entry point and retain private GitHub visibility with authentic history |

## Clarification closeout

The user's foundation review is reflected in all three baseline documents and
their README. SSO is the authentication direction, with Entra ID as a provider
candidate; ownership/provisioning, group synchronization, and active-operation
revocation still require implementation contracts. No SSO or runtime access
configuration was changed. The isolated checker fixture no longer inherits
unrelated roadmap slices. `./scripts/validate.sh` passed governance lint and all
32 tests. Version: N/A; commit: uncommitted.

## License and README closeout — 2026-09-26

- Planned: A short README and the user's selected Apache-2.0 license, committed
  to the existing private GitHub repository.
- Shipped: The official license text, license metadata inherited by all four
  Rust crates and declared by the frontend, and a concise feature/setup guide.
- Not shipped: Public visibility, repository renaming and history replacement.
  The latest user instruction keeps everything private. The isolated privacy
  preparation is retained locally; the original commit dates and history remain.
- New blockers: None for this bounded documentation/metadata change. Any future
  public release requires a fresh privacy review and explicit authorization.
- Docs updated: Root README, continuation record and repository-governance index.
- Validation: Governance lint and 32 checker tests; offline locked Cargo metadata
  confirms Apache-2.0 for all four crates; frontend package/lock metadata agree;
  dependency data is unchanged. License bytes match the
  [official Apache text](https://www.apache.org/licenses/LICENSE-2.0.txt).
  Context7 and the [Cargo Book](https://doc.rust-lang.org/cargo/reference/workspaces.html)
  confirm member inheritance. No runtime code changed or service was restarted.
- Version: N/A; this is not an application release.
- Commit: Uncommitted during validation; commit/push to private `main` is included
  in this assignment. See the containing Git history for the resulting revision.
