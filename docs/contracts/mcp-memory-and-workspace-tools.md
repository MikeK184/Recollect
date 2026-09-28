# Agent memory and workspace MCP tools

Status: accepted

## Direct host setup (2026-09-28 clarification)

Normal remote MCP is the primary browser setup path. Codex and Claude Code can
connect directly to the existing HTTP endpoint with a Recollect device bearer
credential. The signed-in browser may run the existing start/approve/poll/finish
pairing sequence on the user's explicit Create access token action. The credential
is shown once in component memory and can be revoked under Devices; copied host
configuration refers to an environment variable or a host-supported OS credential
helper. For local Codex on macOS, its standard `http_headers_helper` reads the
selected URL's authorization-header JSON from Keychain through `security`.
The secret is entered at a hidden native prompt, never embedded in copied config,
shell commands, arguments or history. Environment setup must give an executable
hidden-prompt/export/launch sequence and distinguish variable names from values.
It inherits the account's current
Brain grants, as existing device credentials do. A configured endpoint selects
one Brain; this is not a claim that the token is restricted to that Brain.

The companion is optional for local repository discovery and session hooks.
Direct MCP can recall and deliberately contribute memory without a companion,
but does not capture a host's conversation automatically. Recollect currently
does not implement MCP OAuth discovery/authorization; host support for OAuth
does not supply that missing server behavior. Do not label manual token setup
as OAuth or a published plugin.

## Source

[ADR 0010](../adr/0010-agent-memory-mcp.md), the full authorized product goal,
[workspace scope](evidence-workspace-scope.md), [retrieval](retrieval-graph-fusion.md),
[claims](memory-claims-and-time.md), [review](memory-review-and-corrections.md),
[autonomous maintenance](memory-autonomous-maintenance.md),
[capture](evidence-session-capture.md) and [managed calls](mcp-runtime-and-credentials.md).

## Contract

### Transport and fixed destination

`/api/brains/{brain}/mcp/agent` serves rmcp Streamable HTTP, with legacy session
mode disabled, bounded JSON responses and the existing origin/host boundary.
Every request requires a current paired device and Brain read access before
discovery/initialization/dispatch. Browser cookies cannot become agent authority.
Use existing app handlers/transactions for every tool, including their current
device/account/grant checks. No actor, token, endpoint or Brain argument exists.
No response cache or MCP session stores authentication. Refuse protocol features
not implemented, including model sampling and generic server-to-client execution.

Requests are at most 256 KiB and responses at most 1 MiB. Permit at most 16
concurrent agent requests per server and use a 60-second request deadline. Limits
return explicit errors; output is not silently truncated into malformed JSON.
Do not retry mutations on transport failure. Return safe application error codes
as MCP tool errors and distinguish unknown tool/schema errors from execution
errors. Never include transport diagnostics, authorization headers or raw driver
errors in model-facing output. Reuse the pinned rmcp 3.4.0 dependency.

`recollect-agent mcp-serve` delegates to a sibling `recollect-mcp-bridge` binary,
keeping the capture hook small. Launch accepts a fixed Brain UUID or the existing
nearest workspace selector, directory and paired endpoint/profile. The bridge
does not follow a changing host CWD into another Brain. Optional capture setup
fixes the original launch Brain/task/device. OS-store failures are explicit;
credentials never appear in configuration, arguments or stdio protocol output.
Root Vault credentials are excluded from the helper environment; Vault is not
required to use this bridge.

### Catalogue and handler reuse

Publish stable, described tool schemas using the existing typed API shapes.
Reject unknown arguments and model-selected destinations. Required scope and
idempotency fields are explicit. The catalogue describes these operations:

| Tool family | Behavior |
| --- | --- |
| `workspace.list` | Current repository/area/environment inventory and own paginated tasks/checkouts; metadata only |
| `workspace.start_task`, `workspace.set_scope` | Create independent task/child or change its future scope, then refresh context |
| `workspace.inspect_task`, `workspace.begin`, `workspace.close` | Existing history, immutable operation binding and task closure |
| `workspace.refresh` | Native-only bounded read-only checkout discovery/publication under the fixed Brain; unavailable over remote HTTP |
| `memory.recall` | Existing exact/lexical/semantic/graph recall with canonical qualifications, provenance and budget/coverage |
| `memory.inspect`, `memory.review_history` | Scoped claim/history and review inspection with current retention and historical qualification |
| `memory.contribute` | Existing evidence-backed claim/procedure creation or revision through a write operation; no fabricated human review |
| `memory.handover`, `memory.handover_status` | Existing scoped multi-contribution generation and status under standing model policy |
| `memory.graph_explore`, `memory.graph_path` | Existing bounded native graph handlers under an immutable operation |
| `mcp.profiles`, `mcp.discover` | Current profile rights and approved cached schemas without starting providers |
| `mcp.call`, `mcp.status`, `mcp.cancel`, `mcp.reconcile`, `mcp.resolve`, `mcp.release` | Existing managed-call admission, history, cancellation, receipt/evidence reconciliation and session release |

Brain readers may discover read/binding operations; writes require writer/admin.
Profile discovery/calls continue to enforce independent Use even for admins.
Discovering a connector does not start it or fetch credentials. There is no
arbitrary shell, URL forwarding, executable configuration, grant administration,
browser review mutation or Erase tool. Models may contribute ordinary evidence-
backed revisions and standing policy performs autonomous learning/maintenance.

### Scope and fresh context

Every memory, graph, write and managed tool operation uses an explicit immutable
operation ID of the existing required kind and same Brain/principal/device.
Models choose the task; the server derives selection from its operation. A second
selection cannot widen it. Historical handles still require current access.
Scoped inspection filters claim revisions and review transitions by applicable
scope and privacy; raw out-of-scope history is never a back door around recall.
Reuse canonical scope/eligibility predicates and existing read handlers.

Task creation/scope change requires a nonempty `context_query` of at most 2,000
characters. After the mutation, immediately read inventory and start a retrieval
binding with `expected_scope` equal to the returned snapshot, then call existing
exact/lexical recall (six records, 16 KiB context). `StartOperation` gains an
optional `expected_scope`; mismatch fails before insertion under the existing
task/Brain transaction lock. Other API/CLI callers remain compatible.

Return the committed handoff plus refreshed context and explicit availability.
If recall, inventory or the expected-scope check fails, preserve the successful
scope-change result and report that context is unavailable/stale. Never return
the previous scope's context or imply rollback. Semantic/graph work happens only
when explicitly requested through recall and current Brain provider policy.
Stable server instructions explain trust/scope once; varying recall is ordinary
tool-result content, without rewriting the system prompt or promising cache hits.

### Host and desktop integration

Generate secret-free Codex stdio settings and Claude `mcpServers` configuration
from the same fixed launch inputs. Managed `capture run` passes those settings
for that launch and preserves other host configuration. Standalone setup prints
copyable scoped configuration without writing personal host profiles. With an
explicit Brain UUID, rendering settings requires neither Keychain access nor a
service connection; authorization is checked when the bridge actually connects.
Generated settings freeze the resolved absolute workspace directory and, when
selected, the absolute `RECOLLECT_CA_FILE` governed by the
[installation contract](operations-local-and-shared.md). Certificate data and
credentials never enter generated settings. Linux Codex settings explicitly
allow forwarding `DBUS_SESSION_BUS_ADDRESS` and `XDG_RUNTIME_DIR` from the host
session so the bridge can use that session's Secret Service. Only their names
are rendered; no session address, credential or broader environment is copied.
UI provides
paired-agent startup/configuration guidance beside workspace tools; no mobile.

When a managed bridge changes its original capture task's scope, create/cache a
new capture binding for that exact scope and atomically update a launch-local
future default in the existing inbox. Existing immutable setups, pinned turns,
delayed tools and child-task defaults remain unchanged. Only later unbound turns
adopt the new default. A failed capture refresh is an explicit coverage gap and
does not undo scope/context changes. Never infer subagent attribution from a
parent session. Managed observations with actual invocation scope belong to slice
26; generic host capture remains honest about original turn scope.

Generic host capture excludes payloads of the generated `mcp__recollect__` tool
namespace and native Recollect MCP commands at local normalization and server
admission. Lifecycle metadata remains attributable; returned memory is not new
independent evidence. A name-based exclusion grants no trust. Ordinary external
tool observations retain their existing capture policy; authoritative managed
observations with invocation scope belong to slice 26.

## Acceptance

Prove real rmcp clients against the production HTTP service and native stdio
bridge: discovery without provider startup; memory recall/contribution/history;
scope changes with immediate new context, concurrent children and old operations;
wrong Brain/device/kind, writer and Use denials, stale defaults, archive/revocation,
rejected/expired/erased records with useful permitted controls. Exercise a real
managed call through the bridge, cancellation/recovery without implicit replay,
bounded/invalid requests and unavailable services.

Use real Codex and Claude hosts in owned fixtures with synthetic model providers,
without personal auth/configuration or paid calls. Verify advertised tools, actual
calls and context reaching the next model turn. Prove future capture default
changes preserve already pinned turns/tools and ambiguous attribution. Verify
desktop guidance and native setup, the unchanged fast-hook bound, relevant
workspace/Clippy/API/web tests and `./scripts/validate.sh`. Recheck normal data,
recall and health without importing fixtures before archive.

## Explicit Deferrals

Managed observation capture belongs to its named successor. No OAuth provider,
host-global memory destination, human-review impersonation, generic filesystem
reader, automatic executable skills, direct external-source writeback, mobile or
customer deployment. Model context already sent to an external host cannot be
remotely erased; new requests always enforce current memory policy.
