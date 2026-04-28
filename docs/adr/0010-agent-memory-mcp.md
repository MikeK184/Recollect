# Agent memory MCP transport and authority

Status: accepted
Date: 2026-09-22

## Why

The authorized first usable product exposes memory/workspace operations to coding
hosts. Catalogue and managed execution already exist. Adding another writer or
copying retrieval policy into a host would create competing authority. The
[foundation](../foundation/techstack.md#identity-workspace-and-mcp-coordination)
selects rmcp, paired identity and immutable operation scope.

## Decision

Expose a Brain-addressed, stateless rmcp Streamable HTTP service using current
paired-device authentication on every request. Its bounded tool catalogue calls
the existing application handlers. A native stdio bridge loads its paired
credential from the OS store and forwards to that service through rmcp. Brain,
endpoint and credentials are launch configuration; tools cannot replace them.
Reuse the existing SDK and HTTP adapters; do not add a second memory engine.

Task/subagent and operation IDs are explicit inputs. A scope change returns
fresh inventory and actual recall under the new immutable scope. A concurrent
later change cannot silently substitute its scope during refresh. Dynamic memory
is returned as tool content, with stable server instructions. Host capture keeps
existing turn/tool bindings; a managed launch may update only its future default.

Agent contributions keep device provenance and pass existing canonical write,
rejection and evidence policy. MCP can inspect qualified review/history; it cannot
impersonate browser human-review or administer retention/Erase. Autonomous
maintenance remains governed by standing Brain policy. No human review queue is
introduced. Existing browser interventions remain available.

## Consequences

The remote service has no authentication-bearing MCP session cache. Device or
Brain revocation applies to discovery and every subsequent call. Managed tools
still need separate Use permission. A transport interruption does not imply a
write failed; mutations use existing idempotency where supported and are never
automatically replayed. Memory already delivered to a host cannot be retracted;
new requests enforce current retention/erasure and carry availability deadlines.

The [contract](../contracts/mcp-memory-and-workspace-tools.md) defines concrete
tools, scope and host behavior. The [mapping](../mappings/mcp-tools-2026-09-22.md)
records interfaces and actual proof separately.

## Supersession

N/A: new decision; existing paired identity, canonical memory, capture and managed
execution decisions remain in force.
