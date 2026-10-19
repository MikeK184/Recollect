# 0022: Brain-managed skills and shared agent guidance

Status: accepted
Date: 2026-10-18

## Native harness amendment — 2026-10-18

After the first local slice, the user explicitly expanded the goal to test all
copy lifetimes and implement actual native install/reload for Codex, Claude Code
and OpenCode. The [native contract](../contracts/mcp-native-skill-reload.md)
now governs that successor. Earlier cache-only boundaries below describe the
first delivery; native host-root installation is no longer deferred within this
bounded, owned workspace adapter. The shipped first pack remains historical.

Use explicit workspace native exports, ownership receipts, a workspace-wide
scope lease shared across host kinds, and real host refresh/activation evidence.
Same-scope tasks share leases; conflicting endpoint/Brain/environment scopes
cannot concurrently change overlapping native discovery roots. Native files are
workspace-visible exports, not per-session confidentiality. Every generated
entrypoint requires a current scoped canonical fetch before following its body
or references. Current human, host and local instructions retain priority.

Instruction bundles become namespaced native skills with the original AGENTS.md
retained as a reference; local AGENTS/CLAUDE hierarchy files remain untouched.
Native adapters use verified host APIs/watchers where available and otherwise
report the exact reload/restart action. Copying files, starting an unrelated host
or receiving an API acknowledgement does not prove an existing host consumed the
content. Executable learning, scripts and arbitrary marketplace import remain
outside this amendment.

## Decision

The following records the first delivery; the native amendment above governs
its explicitly expanded installation/reload boundary.

Extend the Brain's coordination capability with administrator-maintained,
versioned skill and instruction bundles. The user's October 18 goal explicitly
authorizes research and implementation of shared skills, environment guidance,
optional MCP associations, dynamic consumption, and temporary or persistent local
copies. This is curated distribution; automatic executable skill learning and a
public marketplace remain deferred.

PostgreSQL owns each bundle's Brain, identity, immutable integer revisions,
current enabled state, environment applicability, author and publication audit.
Readers discover bounded metadata; an agent activates a selected bundle by
fetching its entrypoint and only the referenced text resources it needs. The
existing authenticated Brain MCP and HTTP handlers expose this same authority.
Do not require downloading an executable or launching an MCP for discovery.

A `skill` has a standard `SKILL.md` entrypoint. An `instructions` bundle has an
`AGENTS.md` entrypoint. These are explicitly manager-published guidance, distinct
from recalled memory and arbitrary tool/source text. They remain subordinate to
current human instructions, host policy and applicable local repository guidance.
The existing rule that memory and ordinary tool output are evidence remains.
No publication, fetch or profile association grants tool execution rights.

The native plugin advertises scoped metadata through its established context
path. A stable bundled bridge skill teaches model-driven discovery and fetch.
MCP-only hosts can use the same tools deliberately without an automatic-hook
claim. This works during a conversation without requiring native skill-directory
rediscovery. Optional CLI save/discard writes only plugin-owned, endpoint/Brain/
version-isolated local cache directories. Temporary saves require an explicit
session identity; persistent saves require `--persist`. Neither mode registers a
native host skill or overwrites/merges repository `AGENTS.md`, `CLAUDE.md`, host
configuration, or an existing skill. Removing files cannot retract conversation
context already delivered to a host.

Initial bundles contain Markdown/plain-text instruction and reference files.
Reject unsafe paths, executable content containers, host permission/frontmatter
overrides and dynamic shell interpolation. No shell, installation hook, automatic
script execution or native host configuration interpreter runs during fetch/save.
Ordinary integer revisions support provenance, optimistic update and optional
historical reads. Preserve [ADR 0003](0003-product-runtime.md): no product digest,
strict pin gate or mandatory client/server version handshake is introduced.

## Why

[Official research](../mappings/managed-skills-host-consumption-2026-10-18.md)
establishes a common consumption model: compact discovery, full instruction
activation, then supporting resources. Agent Skills explicitly permits API
registries and activation tools. Host discovery paths, remote catalogs, name
precedence and reload behavior differ. A fetchable guidance channel provides a
shared baseline while preserving those differences and existing local rules.

The [MCP foundation](../foundation/vision.md#mcp-coordinator-and-vault-integration)
and [plugin ADR](0018-plugin-managed-agent-memory.md) already establish Brain
coordination and native context delivery. Skills add curated reusable working
instructions alongside those capabilities; they do not convert learned memory
into trusted configuration. Semantic instruction merging has no common host
contract, so this delivery keeps sources explicit and independently attributable.

## Consequences

The [managed-skills contract](../contracts/mcp-managed-skills.md) fixes publication,
scoping, limits, failure states, local lifetime and acceptance. The coordination
epic owns the cross-module slice. The Brain Agents surface owns management;
environment and MCP profile selectors reuse canonical Brain identities. No new
model provider, external registry, runtime process or secret store is required.

Disabling a bundle or revoking Brain access blocks subsequent product fetches,
including historical versions. Already exported persistent copies and already
loaded instructions are outside server recall; the UI and runbook must explain
that boundary. The native amendment now governs owned workspace registration
and host refresh. Binary assets/scripts, automatic learning, semantic AGENTS
merging and arbitrary remote-URL import remain explicit deferrals.

## Supersession

Extends the coordination and plugin baseline. Narrows the foundation's blanket
instruction-data separation only for this explicit administrator-published,
typed guidance channel. Recalled evidence, ordinary tool outputs and learned
procedures retain their existing status and authority.
