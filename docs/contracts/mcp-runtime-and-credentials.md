# Managed MCP calls, credentials and instance lifecycle

Status: accepted

### Browser-managed credential provisioning — 2026-10-05

The user's approval of the config/secret-entry concept authorizes an explicit
installation-owner browser provisioning path for central connections. Existing
Vault/environment/OS-store bindings and local/private runner setup stay valid.
`POST /api/brains/{brain}/mcp/connections/{id}/credentials` requires an owner
browser session, Brain admin, open Brain, exact current connection revision,
central placement and a currently approved credential alias. It accepts bounded
masked header or environment values. Existing provider references are selected
through approved connection aliases rather than copied into this endpoint. It
never grants Use/Manage/Share, executes a tool or changes a target.

The installation-local file provider reuses ADR 0003's existing private-file
credential writer, in a separate sibling MCP credential file. Bindings retain
only file/key references, exact connection/alias/runner, allowed header names and
literal prefixes. Files are bounded, atomically written at mode 0600, with
symlink/ownership/permission checks on runtime reads. No values enter PostgreSQL,
manifest/configuration, audit, generated schemas, model input or response. This
is the current installation's local development provider, not a general Vault
replacement or a hardened shared-deployment claim. The UI names this destination
honestly. Environment/OS-store/Vault references remain selectable for existing
operator-managed aliases; browser provisioning never writes to Vault or another
runner. Missing provider values fail closed without fallback.

The central executor uses `RECOLLECT_MCP_CREDENTIALS_FILE` when configured;
otherwise its credential bindings are a sibling of `RECOLLECT_CREDENTIAL_FILE`.
Only the exact selected binding is changed. The runtime continues to resolve
after authority checks, redact exact values and rotate compatible instances.
Secret fields and raw config are cleared on close/success/authority loss and
are never placed in URLs, local storage, caches or diagnostic errors.

HTTP inspection may receive explicit allowed headers transiently under the same
owner boundary. It performs only bounded initialize/list-tools, forbids routing,
cookie/protocol headers and redirects, sanitizes returned metadata with the
exact supplied values and never stores those values. Editing target/auth/config
invalidates the inspection preview. Actual tool use remains independently granted.

## Source

[ADR 0008](../adr/0008-managed-mcp-runtime.md), the
[catalogue](mcp-catalogue-and-profiles.md), [device pairing](platform-device-pairing.md),
[workspace scope](evidence-workspace-scope.md) and the accepted
[MCP foundation](../foundation/vision.md#mcp-coordinator-and-vault-integration).
Routine decisions below are authorized by the full 29-slice implementation goal.

## Contract

### Submission, authority and fixed scope

`POST /api/brains/{brain}/mcp/calls` accepts `request_id` (caller UUID),
`profile_id`, `connection_id`, `tool_name`, object `arguments`, `environment_id`,
optional `operation_id`, non-secret `client_session_id` and `timeout_seconds`
(1–3,600, default 300). The request ID is unique per Brain and principal. Repeating
the same request returns the same current disposition; changed input is 409.
It never creates another provider attempt. Once arguments expire, the ID remains
reserved and a POST replay returns 409 `mcp_replay_expired`; use GET for its retained
disposition. Erased arguments cannot be compared without keeping their contents
or introducing a digest. Reject unknown routing fields.
Arguments are at most 32 KiB, depth 16, and must match the approved input schema.
Reject recognized inline secrets before persistence. These checks are not a
universal detector of secrets embedded in arbitrary strings.

Reuse the catalogue's explicit Use, enabled account/device, current Brain access,
profile membership, environment, enabled connection/definition and configuration
checks. Knowledge, admin, Manage and Share alone do not authorize calls. Device
requests require their owned immutable `tool` operation; browser requests can
use an owned operation or explicitly select an environment. Retain the operation's
full validated scope, actor/device, profile, connection, approved configuration
revision and selected target before queuing. Changes to a task default cannot
relabel a queued or running call. A caller session UUID only partitions that
principal's runtime reuse; it never grants authority.

Recheck the same authority after startup and immediately before the durable
`running` transition. Configuration changes during a wait make that call fail
before dispatch; never silently route it to a new target. The pre-send transaction
defines the ordering with revocation. Already-dispatched work may finish under
its original bounded lease. Revocation denies new calls and credential renewal;
it does not imply remote cancellation or revocation of an already-issued secret.
Returning retained output also requires current Brain and profile Use authority.

### Durable dispositions and APIs

Persist calls separately from retryable jobs:

- `queued`: authorized pending request, expires after ten minutes without claim.
- `starting`: one fenced executor owns startup/validation, before tool dispatch.
- `running`: dispatch authorization committed; sending may already have occurred.
- `succeeded`: a complete non-error MCP response was received. This proves the
  connector response, not deployment or truth of everything in its content.
- `tool_error`: a complete response explicitly reports an error; effects may still
  have occurred. It is not permission to retry automatically.
- `failed`: tool dispatch did not occur, or an explicit protocol rejection was
  received. Reason codes distinguish startup, schema, authority and protocol errors.
  A startup failure does not claim the executable itself had no side effects.
- `cancelled`: cancellation was observed before the pre-send transition.
- `unknown`: dispatch may have occurred but no reliable complete result is held.

`GET .../calls` returns at most 50 own calls plus cursor; Brain admins can inspect
metadata across their Brain without an implicit right to output. `GET .../calls/{id}`
returns current metadata and eligible retained result. `POST .../calls/{id}/cancel`
is available to the initiator or Brain admin: queued calls cancel atomically;
active calls receive a cancellation request. Cancellation, timeout, expired lease,
lost transport or crash after dispatch produces unknown completion unless a final
response wins the fenced completion race. No terminal disposition is blindly
requeued. Cancelling one call must not terminate another call's shared instance.

Retain bounded sanitized result JSON (at most 256 KiB) and arguments only for one
hour after terminal completion; prune them independently of minimal call/audit
metadata. Pending arguments are needed only until completion. The UI explains
expired output. Inputs/results are operational state, not automatically accepted
memory; slice 26 governs managed observation capture and its standing policy.
Minimal audit records admission, dispatch, completion, cancellation and subsequent
reconciliation without raw arguments, result, credentials or private paths.
At most 32 pending/active calls per principal and 1,000 installation-wide; return
429 on admission capacity. No model request is needed by execution itself.

### Runner ownership and placement

Central placement executes in the API service's central executor, never on a
companion. Its database runner lease permits one live owner with a random epoch.
The executor remains bounded and separate from capture, graph and model workers.
Each runner permits at most 16 instances and 16 active calls; each instance permits
at most four calls. Capacity waits keep their original bounded queue deadline.
Only a fenced `starting` attempt that has not dispatched may defer for capacity;
deferral clears that attempt and waits at least one second before another claim.
It cannot requeue a running or terminal call.

Local placement requires `runner_reference = device:{paired-device-UUID}` and an
explicitly started `recollect-agent mcp-runner`. The paired device must belong to
the submitting principal. An unavailable local runner leaves the request queued
until expiry/cancellation; there is no central fallback. The companion loads only
its own endpoint/profile credential from the existing OS store. Device setup is
separate from connector secrets and no device credential is sent to a backend.

Authenticated runner routes under `/api/mcp/runner` provide `register`, `heartbeat`,
`claim`, `start`, `defer`, `complete` and `instance` updates. Each claim carries a random
lease token, runner epoch and immutable execution plan. A second active registration
for the same device is 409. Heartbeat every five seconds extends a 30-second
runner/attempt lease. Pre-send `start` rechecks principal, device, Brain, profile,
scope and configuration and commits running before the runner sends the call.
The runner cannot substitute a target/command/credential alias in that transaction.
Completion updates are conditional on the exact attempt token and epoch; duplicate
identical completion is harmless and stale/changed completion cannot overwrite it.
Credential values never cross this queue protocol.

Loss of authority or the runner lease prevents new sends and credential resolution;
the local executor requests cancellation of affected active work and drains its
owned instances. A lost completion upload is retained in a private local outbox
for bounded retry of the receipt only, never of the tool. Server recovery marks
expired starting attempts failed and expired running attempts unknown. A later
valid receipt may reconcile the original unknown attempt, with both events kept.
This includes supported protocol rejection and proven pre-send failure codes;
unclassified failures cannot replace an unknown dispatched outcome. Duplicate
receipt upload records one resolution and never replays the tool.
No recovery procedure kills a PID retrieved from PostgreSQL. Private placement is
explicitly unavailable until the authenticated private-runner slice is delivered.

The private SQLite outbox is bound to the exact server/runner identity, holds at
most 64 receipts for one hour, and uses immediate transactions with full sync and
secure deletion. Outbox saturation or unavailable receipt storage pauses new
dispatch; neither condition permits repeating a provider call.
An explicitly fenced receipt is quarantined until its original expiry, counts
against storage capacity and is not uploaded repeatedly. The outbox identity is
the service origin plus central runner, or the exact service URL and paired device.

### SDK, startup, compatible reuse and shutdown

Use rmcp's Auto lifecycle with its current supported modern version and legacy
fallback; do not add a Recollect-specific handshake or version gate. Stdio uses
literal approved executable/arguments, never shell interpolation. Streamable HTTP
uses the exact approved target, no redirects or ambient proxy credentials, and an
immutable per-instance HTTP client. SDK session-expiry replay is disabled, as are
automatic `tools/call` retries and automatic sampling/elicitation follow-up rounds.
Unsupported input-required/task results have an explicit incomplete/unknown
disposition; they are not fabricated success or an invitation to another provider.

Persist instance identity/ownership before spawn or connect. Admit provider
startup only while the exact attempt lease and startup deadline remain current;
an expired attempt cannot start a provider even if its runner lease is still live.
Serialize concurrent startup for the full compatibility key: Brain,
connection/profile, actor/device,
caller session, selected runner epoch, approved definition/configuration revision
and resolved credential generation. Startup is bounded to 30 seconds. Use the SDK
for protocol framing, lifecycle and request IDs; a narrow transport adapter may
enforce per-message/body limits and suppress unsafe diagnostics. Limit a wire
message/SSE event/JSON body to 1 MiB and retained results to 256 KiB. Output above
those limits is an explicit incomplete result, never silent successful truncation.

On first use inspect bounded live tool metadata (at most 100 descriptors / ten
pages) for requested-tool availability and valid supported schema. Validate the
actual arguments against both approved and live schemas. This is ordinary input
validation, not exact schema hashing or version negotiation. A live schema cannot
approve new tools, model-controlled HTTP headers, targets or credentials. Recheck
on reconnect; cached discovery remains wholly offline. Successful connection alone
is not a successful tool call. Live tool-list changes drain that instance before
subsequent calls; in-flight calls retain their original authorized inputs.

Each active operation holds an in-memory lease until its wait/cleanup completes,
backed by the durable attempt heartbeat. Useful activity is actual tool work;
catalogue polls, credential checks and keepalives do not extend useful-idle time.
After 15 minutes without useful activity and zero active leases, close our SDK
connection. Close stdin and reap owned children, with bounded escalation through
an owned process group. A lightweight parent-pipe supervisor handles executor
death; detached/daemonizing programs are not supported managed stdio connectors.
Normal shutdown drains active work up to its request deadline; forced shutdown
records/recovers unknown completion. A caller-session release prevents reuse and
drains its instances without interrupting other sessions or active operations.
Hosted HTTP services are never terminated; only our transport is closed.

### Credential-provider boundary

The selected runner reads a bounded operator-owned JSON binding file named by
`RECOLLECT_MCP_CREDENTIALS_FILE`; central execution uses the private sibling
`mcp-bindings` file when the override is absent. Paired runners retain their
operator-configured binding boundary. Each entry
binds exact connection UUID, approved alias and runner reference to delivery
mappings. Environment delivery maps approved destination variable names to a
provider reference. HTTP delivery maps approved header names to a reference and
optional literal prefix (for example `Bearer `). Host/routing/cookie/protocol
headers cannot be overridden. A binding cannot alter the approved target.
These files contain references only, never secret values. Models and ordinary
Brain configuration APIs cannot edit provider mappings. The explicit owner
provisioning endpoint may update exact central bindings under the October 5
amendment above.

Supported initial references are `environment` with an exact variable name and
`os_store` with an entry in service `recollect-mcp` and a non-secret account key.
The October 5 browser-managed development provider adds `local_file` with an
exact path and opaque UUID key; its bounded file must be owned by the executor,
private (0600), regular and opened without following symlinks.
Missing/locked/short/oversized values fail before dispatch, without a fallback.
Values must contain 4–16,384 bytes, matching the shared exact-value redactor's
minimum; shorter values cannot provide the promised exact-output protection.
Resolve only after authority is checked and again for each new operation. The
credential-provider interface also exposes expiry/renewability for Vault in slice
24. Existing environment values are process-local; changing `.env` requires an
executor restart. OS-store/reference-file changes are observed on the next call.
Resolved value changes produce a fresh random in-memory generation and drain the
old compatible instance; active old leases can finish. No secret digest is stored.

Construct a cleared child environment with a fixed basic PATH, approved delivery
variables and non-secret `RECOLLECT_MCP_TARGET` / `RECOLLECT_MCP_CONFIGURATION`.
Reject reserved Recollect/device/database variables and loader-injection names as
delivery destinations. Never inherit the parent environment wholesale. The
operator-approved connector adapts this contract to its native configuration.
Values never enter model arguments, catalogue APIs, instance records or audit.
Suppress raw SDK/provider logs and stderr; return stable diagnostic codes.
Sanitize textual/structured outputs using shared capture redaction and the exact
resolved credential values before retention or returning them. Do not return
opaque binary/image/audio payloads as secretly safe text: report unsupported
content with the corresponding incomplete disposition. This is a trusted connector
boundary, not proof against deliberate encoded exfiltration by an approved program.

### Unknown completion and reconciliation

An optional operator-approved `receipt_policies` list extends a definition. Each
entry names an exposed tool, its reserved `operation_id_argument`, and an approved
`receipt_tool` with `receipt_id_argument`. Both schemas must accept those UUID
strings. The original call ID is injected after rejecting any caller replacement.
Receipt lookup is separately approved as a read-only connector contract; MCP
annotations cannot create it. It must return structured `outcome` from
`succeeded`, `failed`, `not_executed`, `unknown`, plus optional bounded evidence text.

`POST .../calls/{id}/reconcile` schedules that lookup as a distinct authorized call,
with its own attempt/lease and the original target/configuration. Target changes
block lookup instead of asking a different service about the old effect. Parse
only the approved receipt contract; retain its result and a connector-reported
resolution alongside the original unknown state. An unknown/interrupted lookup
does not retry the original operation. No policy automatically resubmits the
effectful call, including when the receipt says not_executed.

For connectors without receipts, `POST .../calls/{id}/resolve` accepts an explicit
observed outcome, bounded explanation and an accessible retained source-version
identity. The initiator with current Use or a Brain admin with explicit Use can
record this evidence-backed reconciliation; native agents can do so autonomously.
An explanation follows the source's current retention/erasure availability and is
kept for at most one hour after reconciliation. Its source identity and observed
outcome remain as minimal metadata. Repeated resolution request IDs return the
same observation only while their complete inputs remain comparable; expired
explanations yield a replay-expired conflict rather than retaining a digest.
The source link, actor and resolution remain distinct from the original missing
response and never label the evidence human-reviewed automatically. Unsupported
or missing evidence leaves the original unknown result visible.

### Desktop and companion experience

The existing tool inspector gains explicit Run with schema inspection, bounded
JSON arguments, timeout and immutable profile/environment selection. There is no
per-call human review queue. Desktop users can observe queued/start/running states,
inspect sanitized output or safe failure codes, request cancellation and reconcile
unknown outcomes. Show actual selected runner, configured/unavailable/connected
state, active leases and useful-idle/draining status. Administration does not
expose output without Use. Polling and observed revocation clear stale content.

Native `mcp call`, `mcp status`, `mcp cancel`, `mcp reconcile`, `mcp resolve` and
`mcp-runner` use the same APIs and paired principal. Calls accept JSON from a file
or `-` for stdin, never accept executable/target/credential overrides, and print no secret.
The public `recollect-agent mcp-runner` command executes a sibling
`recollect-mcp-runner` binary. Install both companion binaries together so the
per-event capture executable does not load the heavy SDK execution path.
`mcp release BRAIN SESSION` and desktop End tool session use
`POST /api/brains/{brain}/mcp/session/release`. Runner availability reflects its
bounded last lease; after a stopped or lost executor it can take up to 30 seconds
to expire. A new owner waits for that lease instead of replacing a healthy epoch.
Recollect's full stdio memory/workspace tool server follows in slice 25. Mobile
views are not required.

## Acceptance

Prove actual stdio and Streamable HTTP calls with the SDK through central and
paired-local production paths. Test current and legacy lifecycle compatibility;
successful listing/initialization alone is insufficient. Prove no dispatch for
missing Use, changed scope/configuration, wrong device/runner, revoked authority,
bad schema or unresolved credentials, with permitted controls. Prove no ambient
secrets reach children or output/log/audit; exercise bounded hostile output.

Concurrent first use starts one compatible instance; different principal/session/
credential generations remain isolated. Long/overlapping work survives idle
sweeps and another session ending. Advancing the test clock verifies exactly the
15-minute useful-idle rule without a 15-minute sleep. Verify owned process reaping,
executor death, runner lease fencing, unavailable-local behavior and continued
hosted HTTP service health after our connection closes.

A fixture records an external side effect then loses its response: the attempt
becomes unknown, admission replay never repeats it, and an approved receipt lookup
resolves it without running the effect again. Exercise lost completion upload,
restart around startup/pre-send/completion, cancellation races and retained-result
expiry. Verify desktop Run/progress/errors/revocation and native local-runner flow.
Preserve the normal database; run focused Rust/frontend/API checks and
`./scripts/validate.sh` before archive. Record limits and actual runtime evidence.

## Explicit Deferrals

Vault/private-runner transport and rotation are slice 24; complete memory/workspace
MCP tools and host refresh are slice 25; governed observation capture is slice 26.
General OAuth enrollment, arbitrary shell/image execution, detached daemons,
universal connector adaptation, automatic effectful retries and mobile UI are
outside this slice. These exclusions do not defer any named foundation capability.

## Desktop session observation — 2026-10-04

The bounded runtime read returns up to 50 authorized visible SDK sessions, sorting
raw active states first. `current_configuration` qualifies whether the instance
matches the currently enabled connection revision and approved enabled definition
revision visible to the actor. Stale configuration instances cannot establish
current readiness. Runner lease loss still rewrites active state to `lost`.

Desktop Connections separates these observed sessions from configuration and
recorded successful calls. An absent instance in a bounded read is only absence
in that observation. Remote service processes are not controlled by this view.
Explicit Pause/Enable use updates the existing connection with its original
revision and all other fields preserved; it does not probe or launch a server,
expand tool grants or claim cancellation of dispatched work.

## Stable inline connection editing — 2026-10-05

The Connections inspector edits its existing configuration rows in place.
Identity, navigation tabs, test status and tool-access sections remain present.
Saving never tests or executes a tool. The editor retains masked secret entry,
approved schemas and placement/credential restrictions. A configuration revision
conflict preserves the draft and requires explicit reload; a failed current
authority refresh hides sensitive configuration. Connection configuration and
credential provisioning remain separate operations: a failed credential step
reports the configured connection and permits only that unfinished provisioning
step to retry against its saved revision, without duplicate creation/configuration
mutation. Name/URL/environment/configuration changes invalidate applicable prior
test evidence through the existing canonical revision rules.
