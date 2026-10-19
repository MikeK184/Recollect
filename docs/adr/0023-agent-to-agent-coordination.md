# 0023: Brain-scoped agent-to-agent coordination

Status: accepted
Date: 2026-10-18

## Decision

The user's October 18 goal authorizes a central Recollect controller that helps
Mike's and Paul's agents discover concurrent work in one Brain, detect related
goals and planned file changes, receive push and pull notifications, and negotiate
through their existing CLI/plugin. Implement the controller in Recollect's Rust
server with PostgreSQL authority and native plugin integration. Prove it with
synthetic data on macOS and an isolated Ubuntu container.

Publish a bounded coordination projection from each participating host session:
authenticated account/device, host/session, current task scope, short sanitized
goal, topic labels and repository-relative file intentions. Existing workspace
tasks, operations, transcripts and local checkout paths remain account-private.
The new shared projection is the explicit coordination surface; it does not make
another account's raw task readable. Session presence is a server-timed lease,
distinct from a configured device or its last API use.

The controller finds explainable overlaps using canonical repository identity,
relative paths, access intent, environment and normalized goal/topic tokens.
The first implementation requires no model call, embedding service or agentgateway.
It reports evidence and uncertainty; it does not claim to predict undeclared work.
Conflicts and negotiations are advisory. Agents may discuss and accept a division
of work, but no notification, message or agreement grants execution authority,
changes a repository or locks another agent's files.

Persist notifications and peer messages before acknowledging them. Authenticated
outbound SSE provides prompt delivery; a durable cursor-based inbox provides
recovery and ordinary pull access. Native hooks consume that inbox at documented
host boundaries. A live stream is not proof that a host injected a notification
into an already running model turn. Direct MCP callers retain explicit pull tools.
Do not add arbitrary callback URLs or inbound listeners on user machines.

Expose a narrow A2A 1.0 HTTP+JSON adapter for the controller's advertised
coordination skill. It calls the same application handlers, authentication and
Brain policy as HTTP/MCP; it is not a second identity, task or execution engine.
Advertise only implemented methods/capabilities and verify the wire against the
current official specification. Agentgateway may later front this endpoint for
routing/policy/observability; adopting a proxy does not implement work matching,
inbox durability or native host delivery.

## Why

The accepted [plugin lifecycle](0018-plugin-managed-agent-memory.md),
[MCP authority](0010-agent-memory-mcp.md), [workspace scope contract](../contracts/evidence-workspace-scope.md)
and [team identity contract](../contracts/platform-team-access.md) supply the
identities and immutable scope required for coordination. The existing agent
roster describes credentials and observed use, not concurrent session presence.
The [official-source research](../mappings/agent-coordination-a2a-research-2026-10-18.md)
separates A2A transport/task interoperability from Recollect's application logic.

## Consequences

The [coordination contract](../contracts/mcp-agent-coordination.md) fixes the
projection, leases, matching, messaging, replay, privacy, bounds and delivery
semantics. New Brain-keyed tables participate in current authorization, audit,
expiry and canonical Brain deletion. Protocol types are shared across native
HTTP/MCP and A2A translation. Existing private execution runners stay optional;
coordination itself starts no managed connector and uses no model budget.

Peer content is attributed untrusted data. A receiving agent must preserve its
human instructions, scope and tool permissions when deciding whether to act.
Server reconnects may redeliver events; clients deduplicate by event identity.
Membership/device revocation stops new calls and stream delivery. It cannot
retract content already shown to another authorized participant.

## Supersession

Extends ADRs 0010/0018 and the coordination foundation. Adds an explicit shared
coordination projection without changing account-private workspace-task access,
the stateless MCP authentication boundary or independent execution grants.
