# Workspace discovery and immutable task scope

Status: accepted

## Source

[Workspace foundation](../foundation/vision.md#brain-and-scope),
[scope principles](../foundation/engineering-principles.md#capture-scope-at-operation-start),
[evidence epic](../roadmap/epics/evidence-and-workspaces.md),
[collections](evidence-collections.md), [devices](platform-device-pairing.md) and
[runtime posture](../adr/0003-product-runtime.md). The active goal authorizes
routine wire, persistence and local discovery details without source hashes.

## Contract

### Local discovery and repository identity

`.recollect/workspace.toml` has exactly one required string, `brain`: an accessible
Brain UUID or an exact unique accessible Brain name. Reject unknown fields,
invalid TOML, controls, an empty selector or a selector longer than 120 characters.
Use a regular UTF-8 file no larger than 8 KiB. Backend URL and credentials remain
in personal environment/OS-store settings, never the workspace file.

Walk canonical parent directories from the supplied existing directory, choosing
the nearest selector. An invalid/unreadable nearest selector is an error, not a
reason to fall back to an enclosing Brain. Discovery never changes an existing
task. A nested workspace is a boundary even if its selector is invalid.

The companion scans the selected root without following directory symlinks.
Exclude hidden/cache/build/dependency directories, while recognizing `.git`
directories and worktree `.git` files. Scan at most 10,000 directories, depth 12,
200 checkouts, 100,000 directory entries and 30 seconds; record truncation and unreadable paths explicitly.
Nested Git checkouts are included when reached within these bounds. Read local
origin, branch, HEAD and dirty status using bounded Git subprocesses with optional
index writes and filesystem monitors disabled. Never fetch, checkout, extract,
run hooks, modify Git configuration or read ordinary source text. Git errors
produce unavailable metadata rather than empty/clean assertions.

Origins normalize HTTPS/HTTP/SSH and SCP-style syntax to a lower-case host plus
case-preserving full repository path. Strip credentials, a terminal `.git`,
trailing slashes and standard transport ports. Preserve custom ports. Reject
query/fragment, local/file origins and ambiguous paths instead of guessing.
Do not retain the raw origin or subprocess stderr. Same normalized origin in a
Brain resolves to one stable repository UUID; other Brains have separate IDs.
Forks/distinct full paths remain different. An admin may explicitly attach a new
origin to an existing UUID; an origin already belonging to another repository
cannot be silently merged. Alias attachment preserves the original display origin;
at most 100 origins may identify one repository.

Only a paired writer/admin can publish a checkout refresh. The backend validates
canonical origins again. Registrations belong to account/device/Brain/workspace
root; paths and checkout observations are visible only to that account under
current Brain access. Repository identities are shared within the Brain. A full
refresh marks previously registered but unseen paths `not_seen`; a partial scan
does not infer absence. Neither state erases identity/history or proves remote
availability. Changing a checkout's origin updates its next observation, never
an existing task's selected repository IDs.

Bound root/path/origin to 2,000 characters, branch to 256, notes to 100 safe
messages of 200 characters, refresh payload to 2 MiB, repositories to 1,000 per
Brain, workspaces to 50 per account/Brain and registrations to 1,000 per workspace.
Checkout pages contain 100 rows. Canonical refresh, audit and queued Brain
projection commit atomically. No raw checkout is uploaded.

### Task, subagent and operation binding

Every task has a UUID, Brain, account, optional originating device/workspace,
label and optional parent task. A child belongs to the same Brain/account and
gets its own scope snapshot, copied from the parent's current scope unless
explicitly selected. Parent/default/CWD changes cannot mutate child history.
A workspace registration must belong to the same Brain/account. Devices may
create tasks for their own workspace registrations; browser-created manual tasks
can select the account's registrations. Device provenance is recorded separately
for every mutation/operation.

A scope contains sorted unique repository IDs (at most 100), area IDs (at most
100) and an optional environment ID. Each reference must exist with the correct
kind in the task's Brain. Empty dimensions mean Brain-wide for that dimension;
selecting an environment grants no execution rights or implicit profile use.
Store immutable UUID snapshots with labels at selection time. Scope changes
require the observed `base_scope` and create a new snapshot, returning 409 if
another operation changed the default first. This is ordinary stale-editor
protection, not strict version negotiation.

Each operation starts in a transaction that fixes the task's current snapshot
and records an immutable operation UUID, kind, principal/device and start time.
Kinds are context, retrieval, write, capture and tool; this endpoint creates a
binding only, not a completed retrieval/write/tool call. Write/capture bindings
require a writer/admin. Later handlers consume the binding and recheck current
authorization and applicable resource policy. A scope handle alone is never a
credential. One task/subagent cannot be substituted for another operation's
recorded owner or Brain.

The optional `expected_scope` operation-start field rejects a concurrently changed
default before creating a binding. The
[MCP tools contract](mcp-memory-and-workspace-tools.md) uses it to recall exactly
the snapshot returned by a scope change. Existing callers may omit it.

Scope changes return an explicit handoff with previous/current scope IDs and
`fresh_context_required=true`; clients refresh the repository/area/environment
inventory immediately. The [exact/lexical retrieval contract](retrieval-exact-and-lexical.md)
extends the handoff to advertise recall availability. Starting a binding still
does not itself perform recall; native and future MCP consumers call that handler
using the new operation's fixed selection.
All contributed operation scopes remain inspectable for later multi-repository
handover/capture, rather than labeling a session with only its final scope.

All current Brain members may create their own working tasks. Tasks/operations
are private to their account even for a Brain admin. Task mutations retain audit
in the same transaction; temporary working context has no materialized projection
and needs no dummy background job. Archived Brains reject new bindings/mutations.
Close stops new task operations; children remain independent. Keep at most 100
active tasks per account/Brain, 1,000 snapshots and 1,000 operations per task.
Task, scope-history and operation-history pages contain 20 rows. Historical IDs
survive view deletion; new operations fail explicitly if a selected view vanished.
Current grant/device revocation denies reads and new operations. Historical
bindings do not bypass current policy.

### Interfaces and UI

Use `/api/brains/{brain}/workspace` for the repository/area/environment inventory,
own workspace registrations and paginated tasks/checkouts. Its query accepts
`workspace_id`, `checkout_offset` and `task_offset`. Other paths are:

- `POST /api/brains/{brain}/workspace/checkouts`: publish a companion refresh.
- `POST /api/brains/{brain}/workspace/repositories/{repository}/origins`: attach an explicit alias.
- `POST /api/brains/{brain}/workspace/tasks`: create a task or independent child.
- `GET /api/brains/{brain}/workspace/tasks/{task}`: current task and paginated scope/operation history.
- `PUT /api/brains/{brain}/workspace/tasks/{task}/scope`: change only its future default.
- `POST /api/brains/{brain}/workspace/tasks/{task}/operations`: start an immutable binding.
- `POST /api/brains/{brain}/workspace/tasks/{task}/close`: close the task.
- `GET /api/brains/{brain}/workspace/operations/{operation}`: inspect an authorized binding.

The native CLI provides offline `workspace discover`, paired `workspace refresh`
and `workspace list`, plus explicit Brain/task `scope start`, `scope fork`,
`scope change`, `scope begin`, `scope inspect` and `scope close`. Scope commands
use explicit IDs and never infer a new Brain from a changed directory.
`workspace list` selects the calling device's registration at the discovered
root; it does not substitute another device/root's last refresh. An unregistered
root has no selected checkout catalogue while the Brain inventory remains visible.
The browser shows registered repositories, own checkout observations, task/
subagent creation, scope selection, handoff, history, operation bindings, close
and alias management. Show empty, partial, stale, unavailable, error and denied
states without claiming extraction, publication or retrieval occurred.

## Acceptance

Use real local Git fixtures for nearest/invalid/nested workspaces, worktrees,
equivalent origins, dirty/unborn/unavailable states and read-only discovery.
Exercise authenticated database/RLS refresh and task handlers with foreign-ID
denials and authorized controls. Prove simultaneous child defaults and old
operation bindings remain independent, stale updates fail, missing views do not
widen scope, and revocation denies further use. Verify native paired refresh and
browser scope/history flows, desktop/mobile layout, focused Rust/frontend checks
and governance validation before archive.

## Explicit Deferrals

Exact Enola extraction/publication and environment revision manifests follow in
repository publication. Host event attribution/inbox belongs to session capture;
retrieval consumes handoffs in its slices; MCP transport/catalogue/profile use
and actual invocation remain in coordination. No current binding claims those
effects or exposes arbitrary local source files.
