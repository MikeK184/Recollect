# Workspace scope dependency and local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The [workspace contract](../contracts/evidence-workspace-scope.md) governs this
slice. Context7 resolved and queried `/websites/rs_toml` and `/git/htmldocs` for
typed TOML parsing, worktree discovery, origin configuration and read-only status.
Primary interfaces checked:

- [TOML crate, observed 1.1.6](https://docs.rs/toml/latest/toml/)
- [URL parser, observed 2.5.8](https://docs.rs/url/latest/url/struct.Url.html)
- [Git revision/worktree discovery](https://git-scm.com/docs/git-rev-parse)
- [Git status and optional index locking](https://git-scm.com/docs/git-status)

The local executable reports Git 2.50.1 (Apple Git-155). Added compatible `toml =
"1"` and `url = "2"` dependencies, resolved through normal Cargo lockfiles.
No product hashes, release gates or strict wire version negotiation were added.
Git's existing commit IDs remain revision metadata.

Used actual temporary Git repositories/worktrees within Recollect, actual
PostgreSQL application-role RLS, paired native OS-store credentials and isolated
Playwright Chrome. Inspected desktop task/history and checkout screens, plus the
complete mobile layout. The UI harness now builds into its own ignored static
directory so testing can coexist with the normal running application.

## Observations

| Behavior | Evidence |
| --- | --- |
| Discovery | Starting in a repository subdirectory finds its nearest selector. Nested and invalid nested workspaces remain excluded. Worktrees, unborn branches and dirty files are represented. Source, Git config and index bytes remain unchanged. |
| Origins | HTTPS/SSH equivalents converge without credentials. Full fork paths and custom ports remain distinct; numeric SCP paths, IPv6 keys and local/file-origin rejection have focused controls. Explicit aliases preserve repository UUIDs and conflicting assignments fail. |
| Catalogue | A paired writer publishes real checkout metadata. Distinct device paths share repository IDs. Partial refresh preserves unseen paths; full refresh marks them absent. Browser-only refresh is denied. |
| Isolation | Other Brain members, including admins, cannot see a contributor's checkout paths or private tasks. Foreign Brain/resource/parent/workspace/operation IDs fail with authorized positive controls and direct RLS checks. |
| Task concurrency | Two child tasks change scope concurrently without altering parent/default/old operation state. Stale base-scope updates fail. Snapshot and operation history remain inspectable under current authorization. |
| Revocation | Deleted selected views invalidate new bindings without widening scope. Device/grant revocation denies later reads and bindings. Closing a parent does not close existing children. Archived Brains reject new working context. |
| Browser and native | A real paired native process discovers/refreshes checkouts, creates and forks tasks, changes scope, starts bindings and inspects/closes tasks. Browser changes and native operations preserve each other's recorded scope, including request-error recovery and explicit aliases. |

Eight core API/database scenarios and the dedicated signed OIDC scenario passed.
The two focused protocol/discovery tests passed, including exact index/config
preservation. Clippy with warnings denied and frontend type checking passed.
The generated API contains 60 unique operations. The focused native/browser
workspace workflow passed in 12.7 seconds, then a fresh worker drained its queue.
The final existing browser workflows passed: device pairing (9.6 seconds),
evidence (13.6), platform (4.8), and two team/OIDC workflows (11.1). The workspace
case exposed a picker focus timing issue in the test, then an actual tab reset
when changing workspace registrations. The picker now waits for a normal click,
the tab is controlled, and the expanded workspace flow passed in 11.8 seconds.
It also proves that CLI listing selects the discovered device/root, including
an unregistered root and a different, more recently refreshed workspace.
Together these checks cover six browser workflows; this was not one uninterrupted
six-pass run. Fresh workers drained every successful disposable queue.
Governance lint and all 32 checker tests passed. Normal startup applied migration
006, built the UI, and served the workspace API schema. Actual readiness and
owner sign-in passed; the normal Brain catalogue was empty. Content and scope
integration proof used the isolated databases rather than adding demonstration
data to the normal installation.

The first backend run reached grant revocation and found a test expecting 204
from the existing grant endpoint, which returns 200 with effective access. The
assertion was corrected and all eight scenarios passed. The identified failed
fixture database was removed only after verifying its ownership comment and
absence of retained source artifacts; no unrelated database was removed.

## Translation and Limits

This is local macOS/Git/Docker/native/browser evidence, not a Windows/Linux or
external deployment claim. Discovery is bounded and does not extract or publish
source snapshots. Paths and Git origins are observations, not access grants or
deployment proof. Operation records fix scope for later handlers; they do not
claim that retrieval, capture or a tool invocation completed. Handoffs explicitly
report retrieval unavailable. No customer workspace or reference checkout was
modified by these proofs.

## Follow-up

Exact repository publication consumes the repository identities next. Capture,
retrieval and MCP slices consume immutable operation bindings and current policy.
