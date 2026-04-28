# 0008: Managed MCP execution and uncertain completion

Status: accepted

## Decision

Use the official `rmcp` SDK for protocol handling, stdio and Streamable HTTP.
Add a small shared Rust runtime crate consumed by the server and companion;
it owns SDK connections, bounded I/O, credential delivery and process handles.
It does not link PostgreSQL or independently authorize Brain/profile access.
The server remains the authority for configuration, grants, immutable operation
scope and durable call dispositions. The companion executes local placements
through its existing paired identity. Private runners and Vault extend these same
boundaries in their next slice.

The API process runs one central executor, fenced by a database lease. A local
companion runs an explicitly started executor identified by its paired device;
it claims work over authenticated outbound HTTP. Dispatch and the final pre-send
authorization are separate transactions. Persist a call's `running` transition
before sending `tools/call`. Calls are not generic retryable background jobs.
After uncertain dispatch, interruption/timeout becomes `unknown`; no automatic
retry occurs, even when a remote annotation describes a tool as read-only.
Approved receipt lookup or explicit evidence-backed reconciliation can record what
was later observed without rewriting the original uncertain attempt.

Serialize startup per compatible instance, retain active-operation leases, and
stop only instances owned by the current executor. Compatibility includes Brain,
connection/profile, principal/device, caller session, approved configuration and
resolved credential generation. Use ordinary UUIDs and value comparisons in
memory; introduce no product hashes or custom protocol version gates. SDK Auto
lifecycle handles current and legacy peers. Fifteen minutes without useful work
permits idle shutdown only after the last operation lease ends. Configuration or
credential changes drain old instances while new work uses compatible instances.

Approved credential references resolve on the selected runner from environment
or OS store. Child environments are constructed explicitly; paired credentials,
database secrets and unrelated environment values are never inherited. Stdio
children run under an owned supervisor/pipe lifetime with bounded cleanup. The
runtime does not signal recorded PIDs after a restart or terminate external HTTP
hosts. SDK/provider diagnostics are excluded from normal logs; retained results
are bounded and sanitized before entering the application response boundary.

## Why

The user-authorized full product goal and accepted foundations require actual
central/local/private execution, startup locks, compatible reuse, credentials,
long-call leases and recovery. The [catalogue contract](../contracts/mcp-catalogue-and-profiles.md)
deliberately starts nothing. The [runtime contract](../contracts/mcp-runtime-and-credentials.md)
resolves the next slice's behavior; the [dated mapping](../mappings/mcp-runtime-2026-09-22.md)
records current SDK/specification and read-only Cognee/Atlas findings.

Current MCP supports both a modern per-request lifecycle and legacy initialized
sessions. Protocol cancellation cannot establish whether an external effect
occurred. SDK reuse is appropriate; treating a dropped future as completed
cancellation or a transport reconnection as safe call replay is not.

## Consequences

Execution has a separate durable queue and state machine, with metadata audit,
fenced runner/instance identity and short result retention. A process restart may
leave an honest unknown result. Autonomous receipt checks are supported when an
operator has approved that connector contract; otherwise an agent or person can
record retained evidence. No routine per-call human review is introduced.

This is a trusted internal connector runtime, not a shell sandbox or marketplace.
The operator approves implementations and credential bindings. A configured
connection remains unverified until a real call succeeds. The browser exposes
progress, results, interruption and reconciliation; full memory/workspace MCP
tools and managed evidence capture remain slices 25 and 26.

## Supersession

None. Extends ADR 0003 and the accepted MCP foundation without changing their
ownership, permission or no-hash development posture.
