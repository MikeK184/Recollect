# Brain-managed skills and shared instructions

Status: accepted
Date: 2026-10-18

## Source

The user's October 18 request authorizes architect/reviewer research and complete
implementation of dynamic shared skills and environment instructions.
[ADR 0022](../adr/0022-brain-managed-skills.md), the
[Brain coordination foundation](../foundation/vision.md#mcp-coordinator-and-vault-integration),
[team access](platform-team-access.md), [workspace scope](evidence-workspace-scope.md),
[MCP transport](mcp-memory-and-workspace-tools.md), and
[plugin lifecycle](mcp-plugin-session-memory.md) govern this extension.
[Host research](../mappings/managed-skills-host-consumption-2026-10-18.md) is evidence,
not an additional authority source.

## Native harness extension — 2026-10-18

The user's follow-up explicitly authorizes native install/reload and complete
lifetime testing. The [native adapter contract](mcp-native-skill-reload.md)
extends this contract. Cache-only `save` remains distinct from native `install`;
the earlier shipped pack is preserved. Native host-root registration is now in
scope under owned workspace exports, shared scope leases and real host proof.
All publication, canonical eligibility, format and execution-grant boundaries
below continue to govern the successor.

## Contract

### Ownership, publication and durable state

A Brain administrator creates, replaces or disables bundles. Brain readers can
list and fetch applicable enabled bundles. Ordinary writers do not gain
publication rights. Skills use existing account/device authentication, current
Brain membership, archived/deleted Brain fences and RLS. Every read validates its
Brain and applicability before returning content; a guessed ID is not authority.
Only an authenticated administrator browser session publishes through the
manager HTTP surface; paired-device requests and MCP expose no publication
authority, even when their account is a Brain administrator.

PostgreSQL stores a canonical current bundle identity and immutable integer
revisions containing complete metadata and file contents. Publication, head
update and attributable metadata-only mutation audit commit atomically.
Historical revisions remain attributable and cannot be overwritten. Concurrent
updates require the expected current version and yield a conflict on mismatch.
Version numbers are ordinary audit history, not a strict version/pinning gate.
No product digest or compatibility handshake is required.

Stable names are unique within a Brain across global/environment bundles.
A duplicate name is an explicit conflict; no same-name precedence or semantic
merge is inferred. Stable UUIDs distinguish bundles across Brains. Updates are
complete replacements, including files and associations. Disable is a normal
versioned update with `enabled=false`, not deletion or a promise of remote
content erasure. Brain deletion includes controlled bundle/history rows through
the existing canonical deletion transaction.

### Shared types

The shared protocol owns these JSON fields; UUIDs serialize as strings and kind
values use lower case. File content is UTF-8 text.

```text
ManagedSkillKind = "skill" | "instructions"
ManagedSkillFile = { path: string, content: string }
ManagedSkillInput = {
  name: string, description: string, kind: ManagedSkillKind,
  files: ManagedSkillFile[], environment_id: UUID | null,
  profile_ids: UUID[], tags: string[], enabled: boolean,
  expected_version: integer | null
}
ManagedSkillSummary = {
  id: UUID, brain_id: UUID, name: string, description: string,
  kind: ManagedSkillKind, version: integer, environment_id: UUID | null,
  profile_ids: UUID[], tags: string[], enabled: boolean,
  file_paths: string[], updated_at: timestamp
}
ManagedSkillBundle = { skill: ManagedSkillSummary, files: ManagedSkillFile[] }
```

`expected_version` is absent/null on create and mandatory on update. Unknown
JSON input fields are rejected. At most 64 files, 128 KiB UTF-8 per file and
512 KiB total content are accepted. Names follow Agent Skills' 1–64 lower-case
ASCII letter/digit/hyphen convention: no leading/trailing/consecutive hyphen.
Descriptions are nonempty and at most 1,024 characters. At most 16 distinct
profile IDs and 32 distinct nonempty tags of at most 64 characters are accepted.
All referenced environment/profile IDs must belong to the same Brain.
A Brain may hold at most 64 bundle identities, including disabled bundles; the
one-page catalog never silently truncates. Query is at most 200 UTF-8 bytes.
The sum of path bytes per bundle is at most 2,048, bounding catalog manifests.
Each fully serialized summary, including metadata and file manifest, is at most
12 KiB of JSON; combined authored fields must remain within this budget.

A `skill` requires root `SKILL.md`, with valid YAML frontmatter whose `name` and
`description` match the published metadata. Supported fields are `name`,
`description`, `license`, `compatibility` and `metadata` (string-to-string map).
Optional `license` is at most 1,024 characters and `compatibility` at most 500.
The `metadata` map has at most 32 entries, keys at most 64 characters and values
at most 1,024 characters. Reject other fields,
including host permissions, `allowed-tools`, hooks and context/agent switches.
Use a real YAML parser, including block scalar support. Reject duplicate
frontmatter/metadata keys and enforce the metadata count/text limits during
parsing, including aliased YAML values. A bundle of kind
`instructions` requires root `AGENTS.md`; it is ordinary Markdown guidance,
not a native host file installation.

Allow `.md`, `.txt` and exact `LICENSE` files only. Reject dynamic shell
interpolation introduced by the adjacent characters exclamation mark and backtick
in any delivered file. File names and resource lookups retain exact case. Enforce case-insensitive
uniqueness and consistent spelling of shared path components to support macOS
(for example, reject `References/a.md` together with `references/b.md`).
Reject file/directory prefix collisions as well. Paths are POSIX-relative, at
most 240 bytes and eight segments. Reject duplicate paths, absolute/drive paths,
backslashes, empty/dot/parent segments, control characters, and NUL. Publication
accepts text values, never symlink/archive entries. Local materialization performs
its own containment/symlink checks, verifies actual staged paths/contents against
the expected manifest before announcing success, and never sets executable
permission bits.
This bounded format does not claim compatibility with every existing skill.

### Scope and MCP associations

`environment_id=null` means Brain-wide applicability. Omitted environment on
discovery returns global bundles only. Selecting an environment returns global
bundles plus bundles bound to that environment. Other environment bundles are
excluded. Applicability is organization within the existing Brain read boundary,
not a new per-environment permission grant. Search `query` matches bounded
metadata, including name/description/tags; it does not load all instruction bodies.

Profile references are advisory associations: show which existing managed tool
profiles accompany the guidance. They never grant Use, start a connection,
provision credentials, approve a definition or silently change execution scope.
Actual calls continue through the existing independent profile Use checks.
Removing an association does not revoke or create any tool grant. An environment
referenced by current or historical guidance remains in use and cannot be removed
as an unused workspace environment; changing the current association does not
erase historical applicability. Canonical Brain deletion removes the full
controlled history through its existing deletion workflow.

Every MCP discovery/fetch requires an existing immutable read-capable
`operation_id`. The bridge fixes the Brain from its authenticated endpoint and
forces environment applicability from that operation. Model-supplied Brain,
endpoint, actor or environment overrides are rejected. An ended task, invalid
operation or other device/account's operation cannot be reused. A later default
scope change does not relabel an earlier operation. Validate current canonical
eligibility at fetch time even if discovery happened earlier.

### HTTP and agent tools

The Brain-relative HTTP surface is:

| Method and path | Behavior |
| --- | --- |
| `GET /api/brains/{brain}/skills` | Metadata-only `ManagedSkillSummary[]`; optional `environment_id`, `query`, and administrator-only `include_disabled` |
| `POST /api/brains/{brain}/skills` | Administrator create; returns complete current bundle |
| `GET /api/brains/{brain}/skills/{id}` | Current bundle by default; optional integer `version`, exact resource `path`, or `entrypoint_only=true`; optional environment selector enforces applicability |
| `PUT /api/brains/{brain}/skills/{id}` | Administrator full replacement, with `expected_version`; returns new complete bundle |

HTTP list/fetch may carry `operation_id` for task-bound consumption. Whenever
present, the server validates it and derives environment from the operation;
conflicting explicit selectors are rejected. Ordinary administrator browser
management can read/edit disabled bundles; device/client consumption cannot
fetch a disabled bundle, even by selecting an older enabled version. Historical
reads require both current head eligibility and requested revision applicability.
An absent version selects latest current; explicit absent history returns missing.
A path fetch retains the summary and returns only the requested file. With no
path, `entrypoint_only=true` projects just the kind-specific entrypoint in SQL;
otherwise HTTP returns the bounded complete bundle. An explicit path selects
that resource when both selectors are present.

`skills.list(operation_id, query?)` returns scoped metadata only.
`skills.fetch(operation_id, id, version?, path?)` returns a bundle whose summary
includes the complete file manifest. No path loads only root `SKILL.md` or
`AGENTS.md` according to kind; an explicit path loads only that exact resource.
Relative resource references resolve through another fetch with the same bundle
ID, selected version and operation. No URL/path fallback downloads another source.
Older clients remain compatible; existing tool schemas and publication paths stay
unchanged. Errors use the existing safe application/MCP error mapping.

Guidance delivery identifies its Brain, bundle, version and applicability, and
states that local/human/host instructions retain priority. Only this typed
manager-published channel supplies reusable guidance. Ordinary recall, imported
sources, observed tool output and learned claims remain evidence data.

### Plugin consumption and local copies

The bundled bridge skill tells agents to discover relevant guidance, fetch its
entrypoint before applying it, and fetch referenced resources only when needed.
The existing native plugin task/prompt hook additionally advertises a bounded
metadata catalog for the current operation/scope. It never inserts every body,
registers an arbitrary MCP, launches a runner or waits indefinitely. Escape
untrusted metadata delimiters and cap output; failures leave normal work/recall
usable with a qualified unavailable state. Direct MCP clients can call the tools
without the plugin but receive no automatic-context-injection guarantee.

The plugin CLI exposes `skills list`, `skills fetch`, `skills save`, and
`skills discard` using its existing endpoint, Brain and credential resolution.
Bound streamed HTTP JSON and local ownership-receipt reads to 2 MiB: valid raw
512 KiB text may expand during JSON escaping.
Fetch/list return data without an installed native skill. Save writes an explicit
local copy under a plugin-owned cache isolated by endpoint, Brain, bundle and
version. List, fetch and save require `--operation UUID`. Default fetch returns
only the entrypoint; `--path` selects a resource, while save downloads the complete bundle.
A temporary save requires `--session TASK_UUID`, equal to the operation
binding's task ID; `--persist` selects a retained copy. These switches are mutually
exclusive. Temporary copies are removed on supported SessionEnd/SubagentStop or
explicit discard. Session closure takes the cache lock and writes an ended-task
fence, preventing delayed/concurrent saves from recreating ended-session content.
Supported ended-session recovery repeats this cleanup. Unknown/modified files,
symlinks and orphan incomplete staging directories are preserved for explicit
inspection; no immediate crash cleanup or removal of local edits is promised.
Persistent copies remain until explicit discard. Discard removes only unchanged,
receipt-owned copies in the addressed endpoint/Brain/device lifetime directory;
it rejects foreign paths and does not provide a force-deletion option.

All saves make a current authenticated eligibility check before fetching. Cache
presence is never authority for fresh automatic activation; normal discovery and
fetch recheck the server. A retained exported copy can be manually read offline,
but must be labeled as last fetched and cannot claim current eligibility. No
background watcher promises to retract exports after server disable/revocation.
The cache-only `save` operation does not install into `.agents/skills`,
`.claude/skills`, OpenCode roots or global host directories. The separate native
adapter may manage owned workspace exports under its extension contract; neither
path modifies local AGENTS files.
A saved cache path is not proof of native host discovery or reload. Deleting a
cache cannot make the host forget content already loaded into its conversation.

### Brain manager interface and failure states

The Brain Agents page has a Skills management tab beside existing agent setup.
List name, kind, version, enabled state, global/environment applicability and
optional profile associations. Administrators can create, edit, publish a full
bundle and disable it. Readers can inspect applicable enabled bundles without
mutation controls. The editor accepts an entrypoint and bounded text references;
it explains compatibility limits and displays validation failures precisely.
Publication success refreshes the canonical list; conflict preserves the draft
and requests a current reload. No unsaved draft is advertised as available.

Loading preserves no stale success claim. Empty explains how a manager publishes
the first bundle. Missing/revoked Brain permission shows no-access and no prior
content. Cross-Brain references, malformed YAML, unsafe resources and oversized
content are validation errors; duplicates/stale writes are conflicts. Disabled,
wrong-environment or missing resources return unavailable rather than falling back
to another version/environment. Network failures do not erase local editor input
or grant offline automatic activation. Render all authored text inertly in the UI.

## Acceptance

- Administrator publication/update/disable and reader discovery/fetch work through
  the HTTP service and manager UI. Writer publication is denied; cross-Brain,
  wrong-environment, stale-version and revoked/disabled fetch cases are proven.
- Valid block-scalar YAML/text resources round-trip; malformed frontmatter,
  permission overrides, dynamic interpolation, duplicate/escaping paths, executable
  resources and size violations fail before persistence/materialization.
- Real MCP initialization/discovery exposes the tools. A read operation lists
  metadata without bodies, fetches one entrypoint/resource and retains exact
  provenance; operation environment and account/device ownership are enforced.
- Plugin metadata reaches its supported context output and model-driven fetch
  uses the same typed channel. No arbitrary memory is promoted to instruction.
- CLI temporary/persistent save, session cleanup/discard and endpoint/Brain
  isolation are proven with filesystem containment and symlink denial checks.
- Focused server/protocol/plugin/frontend tests, regenerated OpenAPI, web
  typecheck/build, governance lint and `./scripts/validate.sh` pass. Native host
  discovery/reload is reported only if actually tested, with version and scope.

## Explicit Deferrals

Binary assets, scripts/executable bundles, host hooks/permission fields,
automatic skill learning, arbitrary URL/Git/marketplace import, global host-root
installation, unsupported universal hot reload, automatic MCP grants, semantic instruction
merging and guaranteed erasure of already-exported or conversation-held content.
These deferrals leave dynamic authenticated guidance consumption, manager
ownership, environment applicability and explicit local-copy lifetime complete.
