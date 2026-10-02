# Plugin-managed coding-host memory

Status: accepted

## Source

[ADR 0018](../adr/0018-plugin-managed-agent-memory.md), the user's October 1
approval of the complete plugin replacement and optional runner flag,
[capture](evidence-session-capture.md), [scoped tools](mcp-memory-and-workspace-tools.md),
[direct auth](mcp-plugin-direct-auth.md) and
[private execution](mcp-vault-and-private-runners.md).

## Contract

### Install and connect

Distribute a host plugin containing its memory/connect skills, native hook adapter,
MCP configuration and platform-built Rust runtime. A packaging command produces an
installable local marketplace/package with its executable included. No Rust compiler
or separate companion installation is required on the receiving machine. Source
checkout and packaged installation are distinct and tested as such.

The bundled runtime's `connect --url URL --brain UUID` uses the existing browser
device authorization flow, or an existing access token supplied on standard input.
Secrets go to the existing OS store, never plugin files, arguments, transcripts or
logs. Persist only endpoint, selected Brain, credential-slot reference and runner
settings. Workspace selectors can choose another currently accessible Brain; invalid
nearest selectors fail rather than falling through. Normal host startup requires no
special wrapper or environment export. Status distinguishes not configured, denied,
offline and successfully connected/capturing states.

The runtime supports an explicit `RECOLLECT_PLUGIN_DATA` directory for managed
installations and isolated tests; otherwise it uses the user's application data
directory. Storage must work without the original build checkout, reject symlinks
and unsafe paths, use private files/directories and retain endpoint/device boundaries.
This extends native storage beyond the original repository-bound development path;
it grants no permission to scan unrelated directories or migrate other credentials.

### Session, capture and retrieval

Host session identity, host adapter, workspace, endpoint, device and Brain determine
the session binding. Two sessions in the same folder have independent tasks and
capture launches. Resume retains the original open task. A documented session-end
closes only its owned task in the detached worker; resume after closure starts a
new task with that task's last validated selection. Existing operation bindings
and queued events remain immutable, including delayed completions. A destination
change creates a new binding and cannot move queued events. Never use the latest session in a folder
as a fallback when identity is missing. Missing/ambiguous identity records a gap.

Session startup creates a task under current access. Capture gets a separate
writer-authorized operation when enabled; disabled capture or reader-only access
must not suppress permitted automatic recall. Shared
Rust normalization, sanitization, SQLite inbox and privacy-aware delivery remain the
capture boundary. Hooks only retain permitted documented event fields, never hidden
reasoning or transcript-file scraping. Host adapters preserve provenance and report
coverage gaps. OpenCode uses its current V2 plugin APIs, not V1 hook assumptions.

Where OpenCode completion events are unavailable, use its documented session
context/wait APIs to observe only completed visible replies from model requests
seen by this plugin. Retain only message identities and the immutable request
binding while waiting. Do not ingest historical context, reasoning or provider
state. The runtime rejects binding references from another task, Brain or device;
ambiguous completion records a coverage gap instead of inferring its scope.

The plugin supplies the current task/scope to the model with recalled context;
memory operations and scope changes use that task through the canonical MCP APIs.
Refresh the task's current scope before each new prompt. Create a new capture
binding for future turns when it changes. Existing turns and delayed tool results
retain their original binding. Independently scoped subagents cannot borrow the
root's current scope. Access and deletion are checked again at server admission.

Before a prompt reaches the model, perform bounded retrieval using the sanitized
prompt and the current immutable task scope. Return at most 8 KiB of memory context,
including source references, state qualifiers and a clear data-only boundary. Never
represent recalled text as system authority. Retrieval has an eight-second total
budget and cannot block normal coding on failure. Empty results are not successful
evidence. Provider use follows existing server transmission policy; no client model
or new model credential is introduced. Automatic recall first attempts semantic
and lexical retrieval under server policy, reserving time for lexical fallback;
context explicitly reports when semantic retrieval is unavailable. Compaction restoration performs fresh scoped
retrieval instead of persisting a second ungoverned memory cache. Recalled context
and the plugin's own tools/transport are excluded from independent capture.

Prompt hooks may now make bounded recall requests and emit additional context;
the underlying capture normalization/commit remains local and network-independent.
Uploads/retries run in a separate plugin-owned worker. Abrupt exit preserves queued
events. Later launches resume delivery, enforce expiry and synchronize deletion
before replay. A detached final drain is bounded; no coding session waits for it.

### Optional execution and other capabilities

`connect --with-runner` explicitly enables a separately supervised local runner;
`--runner-id UUID` selects an already-authorized private registration. Without the
flag, the runtime never starts managed local execution. Stopping/disabling affects
only the plugin-owned process. Keep dispatch grants, fixed approved targets, Vault
delivery, leases, unknown-outcome handling and receipt-only recovery unchanged.

Preserve bounded checkout discovery and published workspace metadata through the
plugin. Preserve deliberate contributions, graph reads, handovers, repository
publication and managed-tool APIs. No mandatory source upload or directory-wide
indexing is added. Server memory processing continues after a host session ends.

### Migration and user surfaces

Agents becomes the single setup flow for the complete plugin. Present optional
runner setup separately from ordinary memory. Retain direct HTTP as a supported
advanced transport, with no duplicate first-party server or double capture.
Recognize legacy setup and explain exact migration; do not silently overwrite host
configuration, erase queues, revoke credentials or trust hooks. Reuse compatible
credentials only through explicit selection and verify them against the endpoint.
Document how old pending captures drain with their original bindings. Deprecate the
old managed launch after native ordinary-launch acceptance passes.

## Acceptance

- Fresh packaged install outside the source/build tree; connect once; ordinary
  Codex, Claude Code and OpenCode startup loads tools and automatic hooks.
- Automatic capture becomes canonical evidence and learned memory; a fresh session
  receives relevant cited context before its model answer, without a recall request
  from the user or a voluntary recall tool call by the model.
- Compaction, resume, concurrent sessions, scope changes and delayed tool completion
  preserve attribution. Recalled text is not ingested as new evidence.
- Outage, abrupt exit, duplicate events, expired policy, revocation, Brain deletion
  and stale queued content preserve privacy and do not block unrelated coding.
- The default launches no runner. The explicit flag enables real approved local/
  private execution; disabling it stops only the owned runner. Existing central
  execution, Vault and graph/memory behaviors remain available.
- Focused native-host proofs, negative/positive authority tests, workspace Rust
  checks, affected desktop tests/build, docs validation and diff hygiene pass.

## Explicit Deferrals

Public marketplace submission, OAuth/SSO redesign and unrelated memory-engine
features are outside this integration replacement. Automatic OpenCode capture and
recall are included. Platform/host compatibility must be reported from actual tests,
not inferred from a manifest or synthetic hook payload alone.
