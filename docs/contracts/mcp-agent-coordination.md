# Brain-scoped agent coordination

Status: accepted

## Source

The October 18 user goal and [ADR 0023](../adr/0023-agent-to-agent-coordination.md)
authorize automatic concurrent-work discovery and peer negotiation. Reuse
[workspace scope](evidence-workspace-scope.md), [plugin lifecycle](mcp-plugin-session-memory.md),
[paired identity](platform-device-pairing.md), [team access](platform-team-access.md)
and [MCP authority](mcp-memory-and-workspace-tools.md). The
[research mapping](../mappings/agent-coordination-a2a-research-2026-10-18.md)
records external protocol facts, not delivery proof.

## Contract

### Authority and sharing

PostgreSQL owns participating session metadata, presence leases, overlap state,
peer messages and recipient inboxes. Every read/write/stream rechecks current
Brain membership, account status and device validity. An authenticated device
may announce and mutate only its own account/device task and coordination session.
All Brain member roles may coordinate; coordination does not grant knowledge
write or managed-tool Use. Coordination is device-only: a browser caller gets 403
`device_required` before any Brain lookup. Beyond that the surface follows the
[admission order](platform-team-access.md#admission-order): a Brain the caller cannot
see, a deleted one included, is 404, and an archived Brain denies admission with 409
`brain_archived`.

The server derives account ID/name, device ID/name, Brain, environment and scoped
repository/area IDs; client display strings never become identity. Tasks/scopes
must belong to the caller and Brain. Announcing requires the exact current scope
ID, preventing a concurrent scope change from publishing an obsolete selection.
File-intent repositories must belong to the selection (or to the Brain when its
repository dimension is unrestricted). Scope changes replace only future presence
metadata and matching; delivered messages retain their original attribution.

`context_repository_ids` is an optional sorted/deduplicated list of at most 100
canonical repositories in this Brain. When the immutable task repository selection
is nonempty, that selection governs and every supplied context ID must be within
it. When the task repository dimension is unrestricted, the shared projection
uses these declared working-context IDs for matching. This narrows coordination
context only; it does not mutate task scope, constrain existing authorization or
assert publication/deployed state. The native worker derives the current Git
checkout identity from CWD and normalized origin against its own workspace
inventory, allowing goal/topic discovery before the first edit. Unknown origins
produce empty declared context and a coverage gap, never a basename match.

Only explicitly published coordination fields become visible to other current
Brain members. Raw task IDs/scope IDs are returned to the owner only; raw task
history, conversation, file bytes, credentials and absolute local paths are never
added to peer responses. Existing workspace tasks remain account-private even to
a Brain administrator. Native hooks publish a sanitized bounded goal and supported
file intentions; missing host/task attribution produces a gap instead of borrowing
the most recent session in a directory. Do not infer human consent from a peer's
message or convert that message into privileged guidance.

### Types and limits

UUIDs are ordinary existing product identities. Integer revisions provide optimistic
concurrency, not a protocol pin or product hash. JSON field names are snake_case.

| Type | Required fields |
| --- | --- |
| `CoordinationSessionInput` | `id` UUID, `task_id` UUID, `expected_scope` UUID, `expected_revision` optional integer, `host_kind`, `label`, `goal`, `topics`, `intents`; optional `context_repository_ids` defaults to `[]` |
| `CoordinationIntent` | `repository_id` UUID, `path`, `mode` (`read`/`write`), `phase` (`planned`/`active`) |
| `CoordinationSession` | `id`, `revision`, server-derived `account_id`, `account_name`, `device_id`, `device_name`, `host_kind`, `label`, `goal`, `topics`, `intents`, `environment_id`, optional server-derived `environment_name`, `repository_ids`, `area_ids`, `status`, `last_seen_at`, `expires_at`; own responses also carry `task_id`/`scope_id` |
| `CoordinationMessageInput` | `id` UUID, `from_session_id`, `to_session_id`, `kind` (`note`/`proposal`/`accept`/`decline`), `text`, optional `reply_to` message UUID |
| `CoordinationEvent` | recipient-local monotonic `sequence`, `id` UUID, `kind` (`overlap`/`message`/`peer_closed`), `created_at`, `expires_at`, typed `payload` |
| `CoordinationInbox` | `events`, `next_cursor`, `gap` boolean, `peers` snapshot; expired cursor reports an explicit gap |
| `CoordinationAnnouncement` | `session`, `overlaps` |
| `CoordinationPeers` | `sessions`, `overlaps`; optional `session_id` query selects the caller's matching context |
| `CoordinationOverlap` | `id`, `peer` session projection, `reason`, `paths`, `tokens`, `basis`, `own_revision`, `peer_revision` |
| `CoordinationMessage` | input fields plus `sender`/`recipient` session projections, `created_at`, `expires_at` |

Event `payload` uses `{type: "overlap"|"message"|"peer_closed", data: ...}`.
Overlap/message data use their typed records; `peer_closed` data is `{peer}`.
Close/ack have no request body and return `{state: "closed"|"acknowledged"}`.

Every embedded peer/sender/recipient snapshot excludes private task/scope IDs,
including when persisted in an inbox; only the direct own-session response may
include those fields. The peers response contains at most the 500 active
participants allowed by Brain capacity.

Allow at most 100 active sessions per account/Brain and 500 per Brain,
128 intentions, 32 topics,
80 characters per topic, 120-character labels, 2,000-character goals and
4,000-character messages. `host_kind` is a supported host label or `other`,
bounded to 40 characters. Reject control characters and over-limit requests;
host-side shortening/redaction happens before upload. Topic normalization trims,
lowercases, collapses whitespace and deduplicates without changing display text.

Paths are UTF-8, at most 1,024 characters, slash-separated and relative to the
canonical repository root. Reject absolute paths, backslashes, empty components,
`.`/`..`, controls and URL forms. A trailing slash explicitly denotes a directory
prefix; otherwise matching is exact. Preserve case; do not collapse different
repositories or infer aliases from matching basenames. Never read/upload file
contents to register an intention. A planned intention is a declaration, and an
active intention is a supported observation/declaration, not proof of completion.

Serialized native events/SSE frames are limited to 4 MiB and native response
bodies to 32 MiB. The native worker pulls at most ten events per page. Server
pages stop before their response bound and advance the cursor only over included
events; oversized requests/results fail explicitly rather than silently skip
content. Existing MCP response limits still apply (1 MiB); large projections
require narrower declarations/pages or the native HTTP/CLI path. A successful
small fixture does not prove maximum-payload performance.
The complete peers/announcement response returns explicit 413
`coordination_response_limit` if it exceeds 32 MiB; it never silently drops peers.
Announcement checks this bound before committing a mutation. The current peers
interface has no pagination or query-based narrowing guarantee.

### HTTP and MCP interfaces

All routes below are beneath `/api/brains/{brain}/coordination` and use existing
device authentication. Native public responses use the shared protocol types.
Retained inbox pull/ack remains available to the currently authorized owner after
presence expiry or session closure; neither operation reactivates presence. New
messages, heartbeat and live SSE require eligible live sessions. This preserves
offline recovery without granting a closed task new work authority.

| Method and suffix | Behavior |
| --- | --- |
| `POST /sessions` | Create/update caller session from `CoordinationSessionInput`; return own session and current overlaps |
| `GET /sessions` | Bounded active peer projections in this Brain; optional own session identifies matching context |
| `POST /sessions/{id}/heartbeat` | `{expected_revision}` refreshes only the owned current lease and returns presence |
| `POST /sessions/{id}/close` | Idempotently close owned session, end active overlap eligibility and notify affected peers |
| `GET /sessions/{id}/inbox?after=N&limit=L` | Own durable ordered events; default 50, maximum 100; read does not acknowledge |
| `POST /sessions/{id}/inbox/{event}/ack` | Idempotent own acknowledgement; preserve the event until retention expiry |
| `GET /sessions/{id}/events` | Authenticated SSE with event IDs and `Last-Event-ID` recovery; no callback URL |
| `POST /messages` | Persist attributed message and recipient event atomically; return its stable identity |

MCP exposes `coordination.announce`, `coordination.heartbeat`,
`coordination.peers`, `coordination.inbox`, `coordination.ack`,
`coordination.send`, `coordination.close` through these same handlers. The native
CLI exposes `recollect-plugin coordination announce|heartbeat|peers|inbox|ack|send|close|watch`
and a readable status including current Brain and authenticated peer identity.
Existing direct MCP remains stateless and does not claim unsolicited host delivery.

Create uses a caller-supplied stable session UUID with no expected revision.
An exact create replay returns the committed session; a conflicting reuse returns
409. Update requires the current `expected_revision`; a changed payload increments
the revision. An exact replay of the previous update may return its committed
result; otherwise stale revisions return 409. Heartbeats do not change content
revision and reject a task whose current scope no longer matches the published
scope; announce the new current selection first. A stable message UUID is the
request idempotency key: identical replay returns the existing result; changed
payload or sender reuse returns 409. Apply a limit of 60 new messages per
sending session in a rolling minute; exact idempotent replays do not consume
that allowance. Retries cannot create duplicate messages/events. Content
announcements/changes, message admission and session closure commit with minimal
metadata audit. Lease-only heartbeats and idempotent delivery acknowledgements
persist their timestamps/state atomically without a separate mutation-audit row
for every tick/read receipt; they are presence/transport state, not evidence or
content decisions. Payloads and credentials are excluded from normal logs/audit.

### Presence, matching and notification lifecycle

Default heartbeat interval is 30 seconds and lease TTL is 90 seconds, calculated
from server time. These are product defaults, not an availability guarantee.
Expired/closed sessions do not produce new overlaps. Clock skew on a user's Mac
cannot extend a lease. Heartbeat cannot reopen a closed session; resume creates
a fresh coordination identity when its underlying task closed. A still-open
expired session may announce its current scope to become active again.

A native background worker may renew only while it has observed a host hook in
the preceding 90 seconds or independently verified the owned host process is
still alive. It must not keep a crashed host present indefinitely. Stale queued
hook data does not refresh this observation time. Long silent tools may therefore
show expired/unknown presence when the host offers no reliable liveness signal;
report that coverage limit. An explicit foreground `coordination watch` is itself
an active participant and may renew until stopped; interrupted watchers expire.

Recompute matching after a committed announcement, scope/intent update or expiry.
Compare only active, currently authorized sessions in one Brain. File overlap
requires identical repository UUID plus exact path or explicit directory-prefix
intersection. At least one write yields `file_write_overlap`; two reads yield
`file_read_overlap`. File overlap remains relevant across different environments
because the same repository path is still involved; show both environment labels.

Topic matching uses exact normalized topic labels or at least two shared distinct
goal tokens of length four or more after punctuation splitting. Ignore tokens
`about`, `after`, `agent`, `also`, `before`, `brain`, `from`, `goal`, `have`,
`into`, `need`, `recollect`, `same`, `some`, `task`, `that`, `their`, `then`,
`there`, `these`, `this`, `using`, `want`, `will`, `with`, `work`, `working`.
It requires a shared repository or shared area, or an
identical explicit environment. Different explicit environments suppress
topic-only alerts; an absent environment is unknown/Brain-wide, never silently
"production". Return `topic_overlap` with the actual matching labels/tokens,
repository/area/environment basis and both declaration revisions. No opaque
confidence score, embedding or external LLM is required. This baseline can miss
paraphrases, undeclared paths, renames and semantic dependencies; report that limit.

Maintain an overlap identity from the ordered participant pair and concrete
reason/resource. Emit once when it becomes active or meaningfully changes; a
heartbeat alone never repeats the same alert. Both peers get attributed events.
When overlap ceases or a peer closes/expires, mark the relationship inactive;
stale notifications retain their original time and show current peer status.
An announcement response includes existing active matches even if their event was
already acknowledged. Coordination is advisory, not a distributed lock.

Store at most 1,000 unexpired events per session and retain events/messages for
seven days. Expired metadata is removed by bounded cleanup; Brain deletion removes
it through canonical deletion. Refuse new messaging with explicit capacity state
instead of silently losing an unexpired message. Retention expiry can produce a
cursor gap; return that fact and a fresh peer snapshot. Acknowledgement means the
client recorded delivery, not that a model read, understood or agreed.

SSE is an optimization over the same ordered inbox. Reconnection and process
restart must not lose durable events. Authenticate stream establishment, recheck
before each delivery and at least once per heartbeat interval, and close on
revocation/expiry. A bounded stream closure permits fresh authentication and
reconnect. A silent stream, disconnect or unavailable server becomes visible
degraded status; the client retains its cursor and performs bounded pull fallback.
Requests cannot hold database transactions open while waiting on a stream.

### Negotiation and native delivery

Peers address session UUIDs, not user-supplied account names. Recipient and sender
must be eligible in the same Brain when sending. `reply_to` must be a retained
message between the same participants. `accept`/`decline` require a referenced
proposal and record the recipient's response; unrelated/replayed acknowledgements
cannot change another conversation. A proposal and reply are inspectable records,
not an automatically enforced file reservation or transferred task ownership.

The plugin binds its watcher and local cursor to endpoint/device/Brain/host/session/
subagent identity. It receives SSE in an owned background process, records events
durably, then drains them at documented prompt, pre-tool, resume/compaction or
context hooks. Pre-tool adapters may extract supported file arguments into planned
intentions; arbitrary shell-command text is not a trustworthy file list. Existing
tool hooks remain local and network-free: enqueue supported intention updates and
read the local inbox there; the owned worker performs announcement, heartbeat,
stream/pull and acknowledgement network work. Preserve the existing tool-hook
timeout. Hook latency is bounded and unavailable coordination does not prevent normal coding.
Never infer child-session identity from an ambiguous parent event.

Render concise notices such as `Paul · Claude Code · paul-mac · infra/production`
followed by the shared path/topic, planned/current state, age and reply action.
Escape terminal controls. Peer content is clearly data, limited to a separate
coordination context budget of 4 KiB per hook, with more available by inbox pull.
Deduplicate local delivery by event UUID. Known authentication/access denial,
Brain deletion or destination mismatch suppresses and removes cached deliverable
payloads; a failed network request cannot be relabeled successful reauthorization.
Canonical Brain deletion also removes queued goals, intentions and pending
announcements for the exact endpoint/device/Brain. A durable content-free fence
prevents late worker writes or new session admission from recreating that data;
deletion and local session creation serialize their lifecycle boundary. Busy
cleanup is explicit and retryable, never a false successful erasure. Unrelated
Brains retain their local data.
Offline automatic delivery requires a still-valid last authorization/presence lease,
otherwise show unavailable coordination and recover through fresh authenticated
pull. Already displayed host context remains outside revocation.
Host-supported extra context and visible
CLI/watch output are separate proof claims; do not promise mid-generation model
interruption when the host provides no such interface.

### A2A boundary and integration failures

Implement the official A2A 1.0 HTTP+JSON binding as a synchronous message-only
adapter to the same controller. Beneath the authenticated Brain coordination
prefix, `GET /a2a/.well-known/agent-card.json` returns the Agent Card and
`POST /a2a/message:send` handles the advertised skill. The card's interface URL
is the absolute Brain `/coordination/a2a` URL; include `name`, `description`,
`version`, `supportedInterfaces` with `protocolBinding: HTTP+JSON` and
`protocolVersion: 1.0`, `capabilities: {streaming: false, pushNotifications: false}`,
JSON default input/output modes, the `recollect.coordination.send` skill and the existing
bearer authentication scheme/security requirement. Public unauthenticated discovery
and remote Agent Card fetching are outside this slice.

The bearer card fields are `securitySchemes: {recollect:
{httpAuthSecurityScheme: {scheme: "Bearer"}}}` and `securityRequirements:
[{schemes: {recollect: {list: []}}}]`; legacy OpenAPI-style discriminator objects
are not the A2A 1.0 wire shape.

Use `A2A-Version: 1.0` and `Content-Type: application/a2a+json`. The request is
`{message: {messageId, role: "ROLE_USER", parts: [{data: {schema:
"recollect.coordination.v1", action: "send", message: CoordinationMessageInput},
mediaType: "application/json"}]}}`. Standard optional `contextId` may carry an
opaque caller conversation label but grants no authority; reject `taskId` because
this adapter creates no A2A task. Require the outer message ID to equal the inner
message input ID, preserving one idempotency identity. The response is
`{message: {messageId, role: "ROLE_AGENT", parts: [{data: {schema:
"recollect.coordination.v1", receipt: CoordinationMessage}, mediaType:
"application/json"}], contextId?}}`; its response message identity is stable
for replay. No legacy `kind`/`type` discriminators appear in the A2A envelope.

The peer message is durably queued through the ordinary native inbox; this
synchronous A2A receipt does not mean the peer has replied. Native coordination
SSE is independent of A2A task streaming. No A2A task state, subscriptions,
artifact service or callback configuration is advertised. Native snake_case DTOs
belong inside the standard camelCase envelope's structured data parts.

A2A roles or client metadata cannot impersonate another account, gain tools,
read private tasks or turn a peer message into executable authority. Advertise no
arbitrary task execution, callback push configuration or cross-Brain federation.
Unsupported operations return the standard unsupported-operation error. Tests
must use actual wire requests against the advertised binding, not only native DTO
unit tests. No agentgateway or model provider is required to run the controller.

A2A HTTP errors use the specification's `error` envelope with numeric HTTP
`code`, canonical `status`, `message` and typed `details`; A2A-specific failures
include `google.rpc.ErrorInfo` with the standard reason and
`domain: "a2a-protocol.org"`. Native controller errors must be translated rather
than returned in their internal JSON shape: a browser caller maps to `PERMISSION_DENIED`
with the native `device_required` message and an archived Brain to `ALREADY_EXISTS` with
the native `brain_archived` message. Missing/unsupported version handling
follows the official service-parameter rules; it introduces no native product
version gate.

Validation failures are typed 400/422; missing/foreign resources do not expose
their contents; unauthenticated/revoked requests deny access; stale/replayed
conflicts are 409, and an archived Brain is 409 `brain_archived`; bounded
capacity is 429; transport failure preserves durable cursor/request identity.
Server acceptance is not proof of host delivery. Outbox retry may replay only
idempotent coordination writes, never external tool effects.

## Acceptance

- Two independent accounts/devices, one Brain, separate native session/task IDs;
  Mike on macOS and Paul in an Ubuntu container discover one another with correct
  user/host/context display and no raw-task/absolute-path exposure.
- Same planned file produces one alert to both; read/read is informational;
  different repositories with identical filenames do not match. Different explicit
  environments suppress topic-only alerts but do not hide shared-file risk.
- Topic/goal overlap is explainable and zero-model-call. Unrelated contexts and
  missing/ambiguous declarations do not produce invented work.
- Both push and pull paths deliver actual peer events. Disconnect/restart/replay,
  acknowledgement, cursor gap, lease expiry, closed/resumed task and simultaneous
  updates preserve ordering, identity, bounded retention and deduplication.
- Proposal, reply and acceptance are attributed to the actual two participants;
  cross-account mutation, foreign reply, cross-Brain addressing and revoked device
  requests/streams are denied. No negotiation changes tool grants or source files.
- Real HTTP, MCP tools, native CLI/watch/hook output and advertised A2A methods
  agree. Installed-host injection is reported only for hosts actually exercised;
  Ubuntu-container proof is not a claim of a second physical Mac.
- Focused protocol/server/database/native integration tests, synthetic Docker
  end-to-end proof, generated API consistency, `git diff --check` and
  `./scripts/validate.sh` pass. Record the exact binary/fixture and failure limits.

## Explicit Deferrals

Public federation, arbitrary remote agent URLs/webhooks, agentgateway deployment,
automatic file locks/merges, remote shell execution, model-based matching, hidden
reasoning/transcript scraping, full A2A feature coverage and a new desktop UI are
outside this slice. Existing Agents roster may link/describe coordination but
must not relabel a device's configured/last-use state as a live session lease.
