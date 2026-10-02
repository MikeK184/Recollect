# 0018: Plugin-managed automatic agent memory

Status: accepted
Date: 2026-10-01

## Decision

The user approved replacing the separate local-companion setup with a Cognee-style
plugin: install, connect once, choose a Brain, then start the coding host normally.
The plugin owns authentication, session/task binding, automatic permitted capture,
durable delivery, bounded automatic recall and context restoration after compaction.
Codex, Claude Code and OpenCode receive native adapters for this same behavior.
Existing memory, graph, evidence, repository, privacy and managed-tool capabilities
remain available. This is an integration replacement, not a memory-engine rewrite.

Reuse the Rust capture/sanitization/privacy implementation as a bundled plugin
executable. No separate companion download, pairing command, bridge configuration
or host-launch wrapper is required. The package owns any local MCP adapter needed
to resolve personal credentials/workspace settings and reach the existing remote
MCP service. Host-required plugin/hook trust and initial account authorization
remain explicit; installation does not bypass either.

The independent local/private execution runner is off by default. Plugin setup
accepts `--with-runner`, with an optional explicit private-runner registration.
Enabling it does not create grants, approve connectors or change Vault rules.
It can remain active independently of a coding session. Ordinary memory use
never starts that runner.

## Why

The prior skills-only plugin and companion-managed capture launch connected hosts
but did not deliver automatic plugin capture and per-prompt recalled context.
The user explicitly approved the complete replacement and preservation of existing
functionality, with a flag for the optional runner. Local code is an implementation
detail of a plugin; a separately operated companion is not required for it.

## Consequences

The [plugin contract](../contracts/mcp-plugin-session-memory.md) owns lifecycle,
scope coordination, packaging, migration and acceptance. Keep the durable queue,
privacy/deletion fences, canonical server admission and scoped MCP tools. Do not
copy recalled context back into evidence or treat retrieved text as instructions.
The old capture launch remains migration compatibility until ordinary-host proofs
pass; it then ceases to be the documented default. Existing queued evidence is
drained with its original authority and is never silently relabeled.

## Supersession

Supersedes the companion-only automatic-capture boundary of
[ADR 0015](0015-direct-plugin-user-auth.md), the silent-capture-only host integration
boundary of the [capture contract](../contracts/evidence-session-capture.md), and
the standalone-companion packaging assumption in the foundations. Their underlying
authentication, scope, evidence, retention and execution guarantees remain in force.
