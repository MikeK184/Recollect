# Automatic host session capture

Status: accepted

## Source

The accepted [capture vision](../foundation/vision.md),
[workspace scope](evidence-workspace-scope.md), [retention](memory-retention-and-erasure.md)
and [autonomous maintenance](memory-autonomous-maintenance.md) govern this producer.
The full product goal authorizes routine integration and local storage choices.
[Interface evidence](../mappings/session-capture-interfaces-2026-09-14.md) records
current host documentation and Cognee integration patterns without adopting its runtime.

## Contract

### Policy and authority

Each Brain has an independent capture policy: disabled initially, permitted event
kinds (`prompt`, `reply`, `tool_result`, `lifecycle`), excluded tool names, excluded
content literals and maximum normalized event bytes (1–64 KiB, default 16 KiB).
Bound each exclusion list to twenty strings of at most 200 characters. Sensitive
paths, recognizable credentials and configured secret values are always excluded
or redacted; policy cannot disable that baseline. Only browser Brain admins change
the policy with its observed change ID. Readers can inspect it. Capture permission
does not grant model transmission, which still needs the event's raw content class.
First-party recall/MCP output is derived context, not independent evidence. The
[agent tools contract](mcp-memory-and-workspace-tools.md) specifies exclusions for
its generated namespace and native commands at both capture boundaries.

Paired writers create a capture binding from their own active `capture` operation.
It fixes the Brain, actor/device, task, immutable scope and host/version. A binding
is an attributable input reference, not a credential. Retain a UUID, current policy
receipt and time locally; no bearer token is stored in the inbox. Archived Brains,
revoked devices/grants and missing selected resources deny new publication. Recheck
current authority and capture policy before accepting any queued event. Task closure
stops new binding creation; previously captured permitted events may drain using
their original binding, subject to current access, retention and deletion fences.

The native setup/launch path resolves the existing nearest workspace Brain selector.
An explicit Brain UUID overrides that default; validate it against current device
access. Neither selection method infers repository, area or environment scope.
It can create a new Brain-wide task or bind an explicitly selected owned task;
repository/environment scope is never guessed from CWD. Scope changes create new
bindings for subsequent capture. A turn keeps the binding observed when it starts,
including delayed tool completion. Child attribution uses an independently bound
task or an explicitly validated inherited snapshot, never the parent's latest default.

### Supported hosts and normalization

The first host adapters consume documented Codex/Claude command-hook JSON from stdin:
SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop, SubagentStart,
SubagentStop, PreCompact, PostCompact, SessionEnd and Interrupt where the host
supports them. Claude PostToolUseFailure/StopFailure can record failed observations
or coverage gaps. Lifecycle markers contain no arbitrary transcript content.

Store a server/local event UUID, binding, host/version, opaque session/turn/agent/tool
identities when supplied, event kind, capture time, sanitized content and explicit
coverage flags. Do not read `transcript_path`, local source files or hidden reasoning.
Raw hook payloads never go to disk or logs. Standard input is bounded to 256 KiB
before JSON parsing; oversized/malformed input produces a safe local gap counter.
Preserve UTF-8 boundaries. Unknown host events/fields do not expand capture implicitly.

`prompt` and `last_assistant_message` provide conversational text. Tool observations
preserve sanitized structured arguments/results and their tool-call ID, distinguishing
failure from success and unavailable output. A reported tool result is attributed
evidence, not independent deployment verification. Exclude tools targeting sensitive
paths, including credential files and private keys. Exclude capture's own transport
operations and native `scope recall` commands to avoid self-referential loops.
Recognize quoted executable paths and decoded JSON arguments at both admission
boundaries; a permitted documentation read may mention those commands in its
output. Whole-payload credential redaction still applies. Redact before truncation. Apply the same
deterministic sanitizer locally and again at server admission; never send raw data
to a model for redaction. Unknown/encoded secrets cannot be universally detected;
explicit exclusions and configured secret matching bound that limitation.

Codex subagent hooks may share a parent session ID. Root prompt turn IDs and explicit
child bindings identify scope; unrecognized turns/subagents produce an
`ambiguous_attribution` marker with no content instead of borrowing root authority.
Claude uses its available prompt/agent/tool IDs; older missing identity is a coverage
gap. Missing replies, hooks, unsupported tools, interruption and out-of-order delivery
are observable. Never advertise complete transcript coverage from these hooks.

### Durable local inbox and delivery

Use a private SQLite inbox with transactional identities and sanitized payloads.
Keep it in the companion's selected data directory, separated by endpoint/device.
Bundled SQLite is a local implementation dependency, not a server database driver.
Use foreign keys, synchronous commits, bounded lock waits, deleted-page clearing and
rollback-journal deletion; do not retain raw content in a WAL after expiry/erasure.

The hook only normalizes, validates its cached binding/policy, pins turn scope and
commits an event. It performs no network or model call. Aim below one second on
the local fixture; the host command timeout is three seconds. Successful hooks
emit no model context or control decision. Failures emit a static diagnostic and
do not block the user's coding work. Counters expose gaps where an event could
not be retained. A stale/missing cached policy cannot authorize new content;
cached capture permission expires after 24 hours without synchronization.

The local UUID is stable through retries. When a native event key is available,
use the binding plus host/session/turn/agent/tool/event identity as a uniqueness
constraint. A repeated key with different content is reported, not overwritten.
When the host supplies no stable delivery identity, retain a local UUID and mark
host deduplication unavailable. Never hash content or invent exact-once host delivery.

Keep at most 5,000 pending events and 64 MiB of pending text per profile. Preserve
existing queued work when full and count rejected arrivals. A separate worker
synchronizes policy/deletions, uploads at most twenty events per pass and records
acknowledgments transactionally. Upload retries reuse event UUIDs. Use bounded
exponential delays up to sixty seconds for transport/server failures; deterministic
denials are visible and do not obstruct unrelated events. A response lost after
server commit returns the original content-free receipt on replay. Conflicting
event identity/content returns 409; expired/erased identities cannot execute again.
Delete acknowledged local text promptly; retain only permitted receipt metadata.
Restart resumes pending events, with no dependence on a final SessionEnd hook.
The uploader refreshes currently accessible Brains and considers at most twenty
pending rows per pass. It rereads pending bodies after privacy synchronization,
so a stale in-memory batch cannot bypass a newly observed deletion. Restored writer
access and enabled policy can retry permission-denied rows after a sixty-second
delay; identity/shape conflicts remain visible until removal or expiry.

### Central evidence, learning and removal

`GET/PUT /api/brains/{brain}/capture/policy` inspect/change policy.
`POST /capture/bindings` creates an idempotent binding from an existing operation;
`GET /capture/bindings` lists the caller's bindings. `POST /capture/events` accepts
one normalized event through a transaction with its receipt, source/evidence,
audit and durable processing job. `GET /capture/events` provides twenty-row pages,
filterable by binding/kind, under current Brain read access. Only a binding's
originating device can publish through it. Browser callers cannot forge device capture.

Admission validates the fixed event/kind/outcome/coverage vocabulary and bounded
opaque identities. Allow five minutes of clock skew around binding creation and
server time; reject later future timestamps or events predating their binding.
Keep captured text only in its canonical source artifact and processed chunks;
the event row holds content-free metadata. While retained, replay compares that
metadata and artifact text without hashing. Replay uses the first admission's
bounded policy snapshot, which expires with the event, so later policy edits
cannot misclassify a retry. A matching native key returns the original event UUID;
the uploader acknowledges the submitted local row with that receipt. Changed
payloads conflict. After removal, opaque identity fences return
a content-free removed receipt without inspecting or restoring the old payload.

Permitted conversational content becomes `raw_session` evidence and tool output
becomes `tool_output` evidence, with exact event provenance and original scope.
Lifecycle/gap markers remain content-free. Captured text uses the existing source
artifact/processing path; canonical source inspection and supporting excerpts work
without a separate memory writer. Autonomous learning inherits captured scope,
rather than relabeling it Brain-wide, and runs only under the Brain's model policy.
Later versions of that source retain the same captured selection for learning.
Models do not execute procedures or invent verified operational outcomes from capture.
Text-model inputs preserve the original lines and carry canonical event/role
attribution beside them. Supported corrections can reconcile across original
events in the same authenticated binding, session and child identity, subject to
exact applicability and current evidence. See [capture reconciliation](memory-capture-reconciliation.md).

The original version's server knowledge time is the event's first receipt, taken
after acquiring the Brain write lock. Host capture time remains observation and
retention time. A later appended version has its own server publication time;
delayed upload cannot make either version visible to earlier knowledge queries.

Deadlines derive from capture time, not retry/upload time. Raw session/tool defaults
remain thirty days; current Brain overrides apply locally and centrally. The local
worker removes expired pending bodies even offline using its last known policy.
Fresh synchronization applies tighter current limits before upload. A disconnected
device cannot know a newer deletion immediately; show its last acknowledged position.

Source/collection erasure expands to affected capture event IDs and queued copies;
the durable journal carries these opaque fences through older-database replay.
Native privacy synchronization includes event fences and clears the capture inbox
before acknowledging the deletion position. Preserve independently supported
evidence and never retain an entire expired conversation via a hidden attachment.
Minimal IDs/times and gap/receipt states may survive according to audit policy.
Opaque host session/turn/tool IDs may remain only in replay keys needed to prevent
duplicate or erased delivery; no labels, paths, arguments or payloads are retained
in those keys. Other controlled event metadata follows its raw-content deadline.
Fences carry event and binding UUIDs plus the optional native replay key, including
when replay precedes the binding/event rows in an older database. Content-free
lifecycle events also expire through the deletion journal. Already expired arrivals
record only their receipt/fence; they never create source artifacts or model work.

### Browser, setup and recovery

Provide Brain capture policy controls, coverage/activity, last successful publication,
source links and safe failures. Configured bindings and queued events are distinct
from a successful connection/publication. Readers cannot change policy. Empty,
disabled, missing companion, offline, partial, denied and expired states are explicit.
Use existing source Erase and retention controls for stored content.
`POST /api/brains/{brain}/capture/devices` accepts a paired device's bounded
content-free report only when it owns a capture binding and still has Brain access.
Reader access can report a denied upload, but cannot publish evidence. Reports
contain Brain queue/denied counts, an explicitly device-wide gap count and an
allowlisted issue code. Browser sessions cannot impersonate a companion report.
`GET` on the same route pages twenty configured companions for Brain readers,
with the latest server-received report and server-confirmed last publication.
After thirty seconds a report is stale: the UI says offline or stopped, without
claiming to distinguish network loss from a closed launcher. Missing reports mean
configured only; accepted event history never stands in for a live queue report.

The native CLI exposes bind/setup, hook, status, drain and a managed run path that
starts upload work and configures the selected host's hooks. Generated host settings
belong to an explicit local directory; preserve personal configs and other hooks.
Existing paired credentials stay in the OS store. Independent drain can finish a
previously interrupted launch. Setup gives one runnable command and an inspectable
Brain URL; it does not require manual acceptance of captured memories.
Hook and local-status commands read only a bounded, credential-free setup file and
the inbox; route them before creating a client or opening the OS credential store.
Use the companion's evidence directory for both publication privacy state and
per-device capture inboxes, so acknowledging one shared deletion position covers
both stores. Generated hook settings and setup files are private and inspectable.
The managed command reuses the setup's immutable binding, observes the installed
host version, inherits the terminal and runs upload passes every two seconds.
After a host upgrade it automatically creates a new binding for the same original
operation and records the new setup path; it does not adopt the task's newer scope
or require the user to pin a host version. An initial bounded sync refreshes cached
capture permission before launching when the service is reachable.
It preserves the host's exit status and finishes with an eight-second bounded
drain; remaining events stay durable for the next launch or independent drain.
Terminal interrupts remain available to the host. A terminated launcher stops
its child. Setup-created tasks remain available for reuse; changing the capture
scope requires a new setup. Existing host configuration and trust requirements
remain in force. A generated definition is not proof that a host executed it.
Codex uses a stable Recollect capture plugin registered through its own marketplace
and plugin installers, as in the Cognee reference. Native setup prepares that
repository-owned bundle; the explicit managed launch registers it in the host's
plugin registry while preserving other plugins and settings. No developer task
silently installs into the user's personal Codex profile. The installed command
captures only when the launch supplies `RECOLLECT_CAPTURE_SETUP`; otherwise it is
a successful no-op. Each launcher supplies its own immutable setup, so another
Codex session or a concurrent Brain cannot inherit a global capture destination.
Claude loads the generated hook file directly through its settings option.
Idle upload passes refresh up to twenty distinct Brains, oldest cached policy
first, so repeated bindings for one Brain cannot starve another Brain's refresh.

## Acceptance

Prove native hook normalization/redaction before persistence, recognizable/configured
secret removal, sensitive-path exclusions, UTF-8/size bounds and unknown input.
Exercise both documented host payload shapes and actual controlled host invocation.
Verify concurrent inbox writes, duplicate delivery, process restart/lost response,
full-queue handling, offline expiry and privacy synchronization. Prove root/child/
concurrent-turn scope, delayed tool completion after scope change and ambiguous
attribution with positive controls. Real API/database proof covers current policy,
foreign IDs, device/grant revocation, artifacts/processing, automatic scoped learning,
30-day/override deadlines, in-flight erasure and older-database replay. Verify UI
states on desktop/mobile, local startup/preservation, focused checks and the validator.
Unverified host compatibility remains visible and cannot count as shipped.

## Explicit Deferrals

Historical transcript import, hidden reasoning, universal host coverage and automatic
execution are outside this contract. Recall injection consumes later retrieval/MCP
slices. Coordinator-owned MCP observations reuse this engine under the
[managed observation contract](mcp-observation-capture.md); host hooks observe only
tool results the host actually supplies. The `managed_tools` policy field defaults
false and selects that independent internal producer, without changing host
binding/device authority or enabling retrospective capture.
