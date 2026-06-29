# Automatic managed MCP observation capture

Status: accepted

## Source

[ADR 0012](../adr/0012-managed-tool-observations.md),
[managed runtime](mcp-runtime-and-credentials.md),
[Vault/private runners](mcp-vault-and-private-runners.md),
[agent tools](mcp-memory-and-workspace-tools.md),
[session capture](evidence-session-capture.md),
[retention and erasure](memory-retention-and-erasure.md) and the
[owning epic](../roadmap/epics/mcp-coordination.md).

## Contract

### Standing policy and admission

Extend the existing Brain capture policy with `managed_tools`, default false
when absent. The existing browser Brain-admin policy editor controls it. Explain
that enabling it publishes sanitized permitted managed results as Brain evidence,
readable by that Brain's knowledge readers. It is independent of granting anyone
profile Use, model-provider consent and host-hook capture configuration. Normal
capture, learning, revision and expiry need no per-item human approval.

Only future calls admitted with capture enabled, `managed_tools` true, the
`tool_result` kind allowed, and a writer/admin knowledge actor are candidates.
Capture denial never denies an otherwise permitted managed execution. Freeze the
policy receipt and the admitted call's actor/device, Brain, profile/connection and
their revisions, approved definition/transport/placement, runner reference,
selected non-secret target configuration, environment, immutable operation/scope,
client session and call ID. Arguments cannot substitute target, identity or scope.
For a browser call without an operation, record explicit Brain-wide repositories/
areas and the selected environment, with no task or operation identifier.

Use current configured-secret matching and the shared deterministic sanitizer
before added capture payloads reach a server outbox or artifact. Runtime receivers
already redact credentials before their existing receipt persistence. Apply
whole-envelope tool/path/content exclusions before truncation, including parameters
and output; preserve UTF-8 boundaries and the Brain's 1–64 KiB event limit. Exclude
first-party recalled context and memory transport from independent evidence, as
with host hooks. Never ask a model to redact. Exact exclusion/redaction is bounded
by recognizable/configured secrets; encoded or unknown secrets are not universally
detectable. A filtered result records a content-free coverage disposition.

### Durable outcomes, scope and publication

Use the managed call UUID as its stable internal capture binding and terminal
event identity. Each canonical reconciliation row UUID identifies its own later
event. Uniqueness is enforced on these existing IDs, without content hashes.
Freeze each outcome/time when it occurs. Terminal `succeeded`, `tool_error`,
`failed`, `cancelled` and `unknown` remain distinct. A tool-reported error carries
its sanitized output; transport/lease failure cannot become tool success. Later
runner or connector receipts and evidence-backed resolution record their own
provenance and link to the original call, without rewriting the original unknown
event. Never copy arbitrary resolution explanation or recall text into new tool
evidence; retain its canonical source reference and outcome instead.

The completion transaction persists its sanitized permitted capture payload and
publication intent with the accepted runtime receipt. Runtime maintenance and
pre-dispatch cancellation persist content-free outcome intents. A durable server
outbox keeps pending event text under its original `tool_output` retention deadline,
default thirty days. Neither restart, retry, delayed upload nor source processing
extends that deadline. Existing runner receipt storage remains capped at 64
receipts and one hour. A receipt that expires before reaching the coordinator is
a visible gap/unknown outcome; the system never invents absent output.

After publication, canonical evidence follows the current Brain retention policy,
anchored to the original capture time, as specified by the retention contract.
An explicit retention-policy extension can preserve still-retained evidence; it
cannot recover cleared bytes or extend the pending outbox's admitted deadline.

Drain at most twenty candidates per pass on a separate bounded capture path;
heavy/model work cannot own this pass's budget. Store attempts, next retry and a
fixed safe error code. Transient publication/storage/capacity errors retry with
bounded exponential delay up to sixty seconds until expiry. Only receipt/evidence
publication is retried. Tool dispatch and side effects are never replayed.
Limit pending retained text to 64 MiB and 1,000 records per Brain. Capacity overflow
preserves existing entries and records a content-free gap for the affected call.
Counts and timestamps expose pending, failed, filtered and missing coverage.

Recheck the original actor's enabled/device state, current Brain writer grant,
profile Use, Brain availability and original scope resources before publication.
Closed tasks may finish an already admitted observation; changed defaults cannot
relabel it. Current capture permission may restrict, never expand, the admitted
policy. A deterministic denial clears pending text and records its disposition;
re-enabling policy does not retrospectively import old calls. Publication commits
one canonical capture event, source/version, audit and durable processing work
together, then clears the outbox text. Replays return the existing disposition.

Managed bindings use host `managed_mcp`, with optional device/operation only when
those were absent in the admitted browser call. They carry the original selection
and managed call linkage. Existing host bind/publish endpoints continue to accept
only supported host/device inputs and cannot create these internal observations.
Actor attribution is the original caller; runner identity is separately recorded.
Runner claims cannot forge the target or another call's evidence.

### Evidence, learning and privacy

Reuse `capture_events`, `evidence::capture_source`, canonical artifacts, chunks,
exact/lexical/semantic/graph eligibility and automatic source learning. Metadata
preserves exact call/observation/receipt lineage, target, scope, timing, outcome and
coverage. Source inspection and model inputs identify reported tool observations;
they cannot imply continuous validity, deployed revision or a human review.
Independent supporting evidence remains independent. Tool-output model transmission
still requires standing purpose/provider/content-class permission.

Reuse capture-event fences and the durable deletion journal. Erasing a captured
managed source fences that managed binding/call against late receipts and queued
copies. Expiry clears each affected event's text and metadata at the applicable
deadline anchored to its original capture time. Privacy closure clears pending server capture text and matching temporary
runtime result/receipt copies; minimal IDs/outcome/audit remain. The runner checks
its bounded pending receipt identities for current removal before resend and
clears matching local bodies. A disconnected runner cannot know a later deletion;
retain the existing explicit device-check-in boundary and one-hour local limit.

On an older-database restore, replayed journal fences can precede the original
call/binding/event rows. Preserve that case without foreign-key dependence on
missing events. The startup privacy barrier and final publication check prevent
erased content from entering evidence, model work, retrieval or graph rebuild.
Neither cleanup nor failure handling revokes any Vault token or lease.

### Product surfaces

Extend the existing capture policy/activity desktop UI for managed capture,
original scope/target/call lineage, source links, partial coverage and current
publication state. Extend call inspection with its observation states and links.
Readers can inspect permitted Brain evidence; policy editing remains admin-only.
Runtime payload/profile permissions continue to govern raw execution history.
Expose bounded pending/error/last-publication diagnostics; configured consent,
accepted runtime completion and published evidence remain distinct.

`GET /api/brains/{brain}/mcp/calls/{id}/observations` returns up to twenty
content-free observation records per page, using that call's current history
authority. Canonical source content follows normal evidence access. The existing
native call inspection and MCP call-history result expose the same publication
disposition. No new manual capture button or separate memory-editing engine exists.

## Acceptance

Run actual central, paired-local and registered-private connector calls through
the existing SDK fixture. Prove useful permitted output becomes scoped source
evidence/recall and policy-governed learning without manual review or a capture
model call. Pair success with tool-error, pre-dispatch failure/cancellation,
unknown completion, late/connector/evidence resolution, current denial and
filtered/oversized/secret-bearing controls. First-party recalled context must not
become independent evidence; permitted unrelated tool output must still publish.

Exercise scope changes with delayed completion and independent concurrent/child
operations; reader execution must not acquire a knowledge write. Verify disabled
and newly enabled policy, Brain/profile/device/runner authority changes, bounded
backlog and temporary artifact failure. Crash/lost response/replayed receipts must
produce one event per identity and exactly one side effect. Prove expiry, source
Erase, pending native receipt removal, late completion after Erase and older-DB
privacy replay, each with permitted retained-evidence controls. Inspect canonical
model provenance and exact retention timestamps.

Run desktop policy/activity/call/source paths and permission/error/empty states;
mobile remains deferred. Run meaningful focused integration tests, workspace,
Clippy, generated API/web and `./scripts/validate.sh`. Record actual runtime proof
and normal-data preservation before archival.

## Explicit Deferrals

Universal host transcript visibility, binary/resource capture, external-source
writeback, automatic tool execution, retroactive import and external deployment.
Backup drills and integrated corpus/capacity evaluation remain named successors.
