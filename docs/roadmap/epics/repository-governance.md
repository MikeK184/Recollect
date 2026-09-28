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
separate assignments. The 2026-09-26 instruction selected Apache-2.0 and
a simpler README while keeping GitHub private at that time. On 2026-09-28,
the GitHub API reports the existing repository as public; the current user
requests peer README research and improved repository description/topics.
This assignment updates documentation and discoverability metadata on that
existing repository. Repository renaming and external deployment remain outside
its scope. The later documentation-skills
assignment belongs to the Developer Tooling epic; it lifts the bootstrap's
deferral for the two requested documentation skills.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `repository-governance-bootstrap` | shipped | adr-backed, contract-backed | pack | Deliver lifecycle checks, roles, Context7, supplied product foundations, and upstream/tooling guidance |
| `foundation-access-and-workspace-clarifications` | shipped | contract-backed | small-fix: bounded documentation clarification and fixture-isolation repair; no runtime change | Clarify dataset ownership, SSO group mapping, managed-operation enforcement, and workspace/developer configuration boundaries |
| `repository-license-and-readme` | shipped | contract-backed | small-fix: established user license choice, README and package metadata; no runtime change | Apply Apache-2.0 consistently, simplify the entry point and retain private GitHub visibility with authentic history |
| `repository-readme-discoverability` | shipped | contract-backed | small-fix: user-requested README presentation and GitHub metadata using existing product authority; no runtime or interface change | Deliver the local README presentation and peer comparison; verify the description and 19 topics on the existing public GitHub repository |

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

## README and discoverability closeout — 2026-09-28

- Planned: Compare Cognee and related projects, improve the README entry point
  and add relevant GitHub description/topics under the user's current request.
- Shipped: Local README with the original brand, factual badges, use cases,
  feature/integration tables and documentation navigation. GitHub's About
  description and 19 relevant topics are applied and verified by a fresh API
  read. The [comparison mapping](../../mappings/github-discoverability-2026-09-28.md)
  records primary sources, topic choices, evidence and limits.
- Not shipped: README/asset publication or the existing uncommitted desktop and
  Compose work. Repository visibility was already public when inspected; this
  task did not change it. Search ranking/indexing and runtime health are unmeasured.
- New blockers: None for the requested metadata and local documentation work.
- Docs updated: Root README and its SVG header, comparison mapping/index, and
  repository-governance epic/index. The old license closeout remains historical.
- Validation: Governance lint and 32 tests; 28 README local links/anchors and
  valid SVG; successful GitHub Markdown rendering and local light/dark visual
  review; `git diff --check`. Existing setup instructions and 207 unrelated
  changed files are preserved. Repository name, visibility, default branch,
  archive/disabled state and homepage match the pre-change API read.
- Version: N/A; documentation and metadata only.
- Commit: Uncommitted; GitHub metadata is live independently of a Git commit.
