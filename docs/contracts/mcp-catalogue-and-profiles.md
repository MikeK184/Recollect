# Approved MCP catalogue and execution profiles

Status: accepted

## Equivalent HTTP schema declarations — 2026-10-06

The user's failing Exa server check establishes the need to accept equivalent
legacy schema declarations during anonymous HTTP inspection. Discovery may
convert a root Draft 7 declaration to 2020-12 only for a bounded audited subset
whose assertion semantics are unchanged. Preserve all assertions, property names,
annotations and descriptions. Reject unknown/dialect-dependent keywords, remote
references/identifiers, nested dialect changes and tuple-array items; do not guess
or silently remove constraints. The final candidate still passes the existing
strict offline validator. Already-approved manifests are unchanged, and direct
manifest import retains strict 2020-12 validation. The health check does not
approve or persist the candidate, start tool execution or confer Use access.

### Installation connector library — 2026-10-05

The approved management concepts add owner-browser-only `GET /api/mcp/definitions`
and `GET /api/mcp/definitions/{key}` for the global `/connectors` library. Reads
return approved/disabled definition metadata and reviewed manifests; they never
connect or execute. The existing owner approval POST remains the canonical import
mutation. Imported host configs are inert drafts; commands are never installed or
run by parsing/approval. A draft must review transport, fixed executable/arguments,
placement, cached tools, configuration schema and credential aliases before approval.
Only actual enabled/disabled states are presented; no fabricated marketplace or
pending-review database state is introduced. Brain administrators continue to use
the existing catalogue and configure only approved definitions. Their tool grants
and runner authority are independent of installation approval.

## HTTP form setup (2026-09-28 clarification)

The installation owner may enter a name and exact HTTP MCP URL and explicitly
request anonymous metadata discovery. The server uses the shared SDK with no
credential, redirect, proxy or tool call, bounded wire sizes, pagination, timeout
and concurrency. Tool descriptions preserve Markdown, newlines and tabs, remove
other control characters and are clipped to the existing 2,000-character display
limit; input/output schemas preserve assertions, with only the equivalent root
declaration conversion above, and must pass the existing offline validators.
HTTPS and exact loopback HTTP follow the existing transport
target rules. Discovery returns a candidate manifest for the existing approval
handler; remote metadata cannot execute commands or grant profile Use. Name/URL
edits invalidate the preview. The form creates the connection through existing
Brain authority after approval; duplicate keys and stale revisions retain their
existing behavior. Advanced manifest import remains available for authenticated,
local and private-runner definitions.

The [managed experience amendment](memory-managed-experience.md) governs the
2026-09-28 managed setup, progressive disclosure and owner connector approval
changes; earlier explicit APIs and stored policies remain compatible.

## Source

The user-authorized full product goal permits routine implementation decisions
within the [MCP foundation](../foundation/vision.md#mcp-coordinator-and-vault-integration),
[runtime ADR](../adr/0003-product-runtime.md), [team access](platform-team-access.md)
and [workspace scope](evidence-workspace-scope.md). This contract governs slice 22
in the [coordination epic](../roadmap/epics/mcp-coordination.md). It adds no per-call
human review requirement and no custom hashes or protocol version gate.

## Contract

### Ownership and approved definitions

The installation operator approves definitions through
`recollect-server mcp-definition-import PATH` using the existing migration
principal and owner identity for audit. The UTF-8 JSON file contains one definition,
at most 512 KiB. Import validates the whole record before atomic insert/update by
stable key. `mcp-definition-disable KEY` withdraws its availability. Both commands
record metadata-only installation audit. Neither command connects to or starts
the implementation. No browser/device/model API registers arbitrary executables.

A definition has a stable ASCII key (1–64 letters, digits, dot, dash, underscore),
name/description, `stdio` or `streamable_http` transport, fixed executable and
argument vector for stdio, approved placement kinds (`central`, `local`, `private`),
non-secret configuration JSON Schema, optional named credential aliases and cached
tool descriptors. A stdio executable is absolute and arguments are literal; no
model substitution or shell interpolation exists. HTTP definitions have no local
command. Configuration reaches a future adapter separately from tool arguments;
runtime delivery and credential-provider bindings belong to slice 23.

Schemas use JSON Schema 2020-12 through the maintained validator with remote/file
resolution disabled. Only local fragment references are supported. Configuration
and tool inputs require `type: object`; output schemas may describe other JSON
types. Configuration rejects undeclared root properties. Validate schema syntax,
schema/descriptor depth and byte bounds; do not fetch `$ref` or descriptions.
Optional icons follow only the explicit inspection boundary below.
Resource-rebasing `$id` and model-parameter HTTP header mappings (`x-mcp-header`)
are unsupported here and rejected instead of introducing an unapproved routing path.
Tool names are unique, case-sensitive ASCII identifiers up to 128 characters.
At most 50 tools, 32 KiB per tool schema, 16 KiB configuration schema and 128 fixed
arguments are allowed. Annotations are display hints, never permission or retry
authority. Tool text renders inertly. No secret values belong in these manifests.

Definition updates are explicit operator approval, reflected on subsequent reads.
Existing connection configuration is revalidated against the current approved
schema; incompatibility is visible and excludes that connection from discovery.
An administrator may still disable an existing incompatible connection while
preserving its routing/settings exactly; new or changed settings must validate.
Runtime reuse/draining on configuration changes is a successor requirement, not
a catalogue claim. Import of identical content does not create duplicate records.

### Brain connections and profiles

The Brain owns connections/profiles; `created_by` records contribution, not an
independent owner bypass. Brain admins provision and edit connections and create
profiles. Named connections select an approved definition, exact target, placement,
optional runner reference, optional approved credential alias, non-secret JSON
configuration and optional environment in the same Brain. Names are unique per
Brain, case-insensitively, 1–120 characters; descriptions are at most 2,000.
Configuration is at most 16 KiB and conforms to the approved schema. HTTP targets
require HTTPS except literal loopback development HTTP, and forbid userinfo,
query and fragment. Stdio targets are bounded non-secret labels/configuration;
they never replace executable/arguments. Central placement has no runner reference;
local/private placement requires one (at most 200 safe characters). A configured
runner/alias is not yet a successfully resolved or connected provider.

A profile has a unique name, description, optional environment and up to 20
distinct connections from its Brain. Brain-wide profiles contain only Brain-wide
connections. An environment profile may also include Brain-wide connections, but
never connections from another environment. Changing a connection environment is
rejected if existing profile membership would become inconsistent. Referenced
environments cannot be deleted or silently changed to Brain-wide; reassign the
referencing configuration before deleting an environment. Disabling only pauses
availability and preserves those references, resource identities and audit.

Create commands support existing idempotency keys. Edits carry the observed
configuration `revision` UUID to reject stale forms with 409; these are ordinary
concurrent-edit checks, not release/version negotiation. Changes and minimal audit
commit together under the existing Brain/account authorization locks. No dummy
processing job or model operation is created. Archived Brains remain inspectable
but cannot be configured or used for execution discovery. Capacity is 100 approved
definitions installation-wide, 100 connections/profiles per Brain and 100 grants
per profile. Capacity errors are explicit, not silent truncation.

### Independent permissions

Current Brain knowledge access is always a prerequisite. Profile grants target
enabled enrolled accounts or exact configured OIDC issuer/group pairs, using the
existing complete, fresh membership snapshot and expiry policy. `use`, `manage`
and `share` are independent booleans; neither implies the others. Effective rights
are additive across direct and current group grants. Brain admins may manage/share
their Brain's profiles as administrative authority, but **never inherit use**.
Creators, Brain owners and the installation owner also need explicit use grants.
Removing a grant reports remaining effective rights and their independent sources.

Manage permits profile name/environment/enabled/membership changes, not editing a
connection's target, executable, credentials or runner. Share permits assigning or
removing the three rights, including an explicit self-use grant. Delegated managers
and sharers may be Brain readers. Only browser-authenticated actions mutate the
catalogue/grants; a paired device may read/discover with its principal's effective
rights. Revoked Brain membership, disabled accounts/devices and stale OIDC
membership are rechecked on every transaction. Direct foreign IDs return 404;
known visible profiles lacking the requested power return 403. UI visibility is
not enforcement. Existing operations authorized before a revocation commits follow
the platform's transaction ordering; this slice starts no running external calls.

### Catalogue, discovery and UI

Use `/api/brains/{brain}/mcp` for bounded summaries: approved definitions for
admins, relevant connection labels/placement/environment, and profiles visible
through any effective right. Admins see all profiles. Managers can select existing
Brain connections; ordinary users see connections belonging to visible profiles.
Full connection configuration and definition configuration schemas have separate
admin-only detail reads. Profile detail includes memberships and, for sharers,
direct/group grants with effective account rights. Credential values are never
stored or returned. Model-facing discovery excludes configuration, executable,
target addresses, runner references, credential aliases and grant inventories.

Paths under that root are `GET definitions/{key}`, `POST connections`,
`GET/PUT connections/{id}`, `POST profiles`, `GET/PUT profiles/{id}`,
`PUT profiles/{id}/grants`, `DELETE profiles/{id}/grants/{grant}`, and `POST discover`.
Grant input identifies exactly one enrolled username or configured OIDC group and
sets the three booleans; all-false removes that subject's direct/mapped grant.

Discovery accepts an explicit profile, optional environment, optional immutable
workspace operation and offset. An environment profile requires that exact selected
environment; selecting knowledge scope never substitutes a profile or grants use.
Devices require an owned `context` or `tool` operation; its immutable environment
must equal the requested environment. Browsers may use the same binding or an
explicit selection. All selected environment IDs must belong to this Brain.
Return deterministic connection-ID/tool-name ordering, at most 20 tools per page,
total/next offset and explicit unavailable-connection reasons. Disabled definitions,
connections and invalid configuration are excluded, while eligible connections
remain visible. Foreign profiles, missing use grants, mismatched scope and revoked
authority fail before any capability metadata is returned.

Discovery reads **only approved PostgreSQL metadata**, opens no network connection,
resolves no credential, launches no process and creates no job/model request.
Descriptors identify the connection and tool separately to avoid name collisions.
They are labeled `approved_catalogue`, with execution state `not_connected`; this
does not establish the eventual backend's availability or live schema. Slice 23
rechecks these same grants and approved configuration at actual dispatch/execution.

The desktop Brain UI lists profiles and independent rights, connection readiness,
environment mapping, catalogue emptiness and setup requirements. Admins configure
connections; delegated managers edit profiles; sharers edit grants. Explicit schema
inspection and discovery show cached tools without a misleading Run/Connected
control. Loading, malformed configuration, stale edit, unavailable definition,
no-use, empty/partial discovery and permission loss are visible. Polling every
five seconds refreshes rights and closes inaccessible/configuration-stale editors;
failed refresh hides stale sensitive detail. No mobile work is required.

### Inline management and connector icons — 2026-10-05

The user approved Connections-style tool-group cards with compact independent
Use, Manage and Share icons beside each person/group name. Green means allowed;
red means denied. Accessible names/tooltips identify both the power and state.
Editing updates values in the existing card/connection inspector with stable
identity, tabs and sections. Manage edits group configuration; Share edits grants.
Effective account rights remain additive; removable direct/group grants are
distinct from inherited administrative/group authority, exposed in a concise
disclosure. The main account icon set always shows effective access; a Share
editor labels separate Direct controls beside the same name, so an inherited
allowed power never silently turns into a red denial. Pending people are marked
as drafts, rendered only in active authorized editing, and discarded on Share
loss/archive. Neither the design nor edit mode grants additional rights.

Configuration and grant commands remain separate canonical transactions. A card
Save applies a changed configuration first, then edited grants sequentially;
it records each successful mutation. On partial failure the draft stays visible,
the UI states which changes saved, and retry sends only unfinished mutations.
An observed conflicting configuration revision blocks that configuration save
while retaining its safe draft for explicit reload. Failed authorization refresh
hides sensitive detail; current authority is rechecked on every mutation.

Only explicit owner HTTP metadata inspection may cache optional server icons.
The SDK's initialize metadata supplies up to three candidate images. Fetch only
same-origin public HTTPS URLs relative to the explicitly inspected target;
reject userinfo, query, fragments, redirects and all private/loopback/link-local,
multicast, reserved or otherwise non-public addresses. Resolve and validate all
DNS answers, pin the connection to those approved addresses, use no proxy,
credentials or cookies, and enforce a total six-second icon budget. Data image
candidates may be validated locally. Decode at most 256 KiB per candidate,
512 × 512 pixels and 4 MiB allocation, accepting PNG/JPEG/WebP only; normalize
to PNG with no source metadata. SVG/HTML and malformed/oversized raster are
rejected. Cache one successful normalized PNG in the approved definition's
existing manifest JSON and expose its bounded data URI in authorized summaries.
Reads never fetch icons. Missing/rejected/failed icons use the generic icon and
never fail successful tool metadata inspection or grant execution rights.

Descriptions and schemas use the bundled safe Markdown/highlighting renderer.
Remote HTML and embedded images remain inert; unknown/oversized languages remain
readable exact text, and explicit Copy preserves the original code bytes.

## Acceptance

Real PostgreSQL/non-owner RLS and API calls prove independent direct/group/admin
rights, foreign IDs, disabled accounts/devices, OIDC expiry/overage, revoked new
discovery, correct positive controls, environment invariants and operation scope.
Exercise malformed/offline schemas, operator import/update/disable, validated
configuration, immutable command/target boundaries, duplicate/replay/stale edits,
capacity/pagination and transactional audit. A sentinel executable and loopback
listener prove that metadata discovery does not start/contact dormant providers.
Desktop tests configure connections/profiles, separately grant use, inspect cached
schemas, preserve errors and clear revoked views. Verify normal data preservation,
generated API, Rust/frontend checks and `./scripts/validate.sh` before archive.

## Explicit Deferrals

Actual rmcp transports, runtime calls, credential resolution, startup/lease/idle
lifecycle and unknown completion belong to slice 23. Vault/private runner transport
and rotation are slice 24; shared memory/workspace tools and host refresh are slice
25; managed observation capture is slice 26. A universal connector marketplace,
shell sandbox, automatic remote schema approval and mobile UI are outside this slice.

### Observed connection result — 2026-10-04

Catalogue summaries may report `last_successful_call_at` from completed succeeded
calls with the connection and connector definition's current revisions, filtered by existing call visibility
and current explicit Use rights. It is historical observation, not connectivity or
current health. Missing authorized history is reported without claiming a connection
has never worked. No tool is contacted by this read.
