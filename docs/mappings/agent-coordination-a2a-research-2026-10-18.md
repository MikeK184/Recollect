# Agent coordination, A2A and native host delivery

Observed: 2026-10-18
Confidence: observed-once

## Sources and Method

Read the current official [A2A specification](https://a2a-protocol.org/latest/specification/),
its [protocol repository](https://github.com/a2aproject/A2A), current
[Codex hooks](https://learn.chatgpt.com/docs/hooks), and
[Claude channels reference](https://code.claude.com/docs/en/channels-reference).
Parallel research used Context7 and official agentgateway examples; the query
details and limitations below distinguish indexed examples from current wire
authority. This is architecture research, not an interoperability test result.

Local evidence: read `crates/server/src/mcp/agent.rs`,
`crates/server/src/workspace.rs`, `crates/server/src/devices.rs`,
`crates/agent/src/plugin_session.rs`, `crates/agent/src/plugin_runtime.rs` and
the native hook manifests. Graft reported its structural graph synchronized;
its missing optional deep/meaning graph was not treated as missing source.
Rust Analyzer symbols successfully resolved `devices.rs`. No research checkout,
customer Brain, credentials or external agent was changed.

## Observations

A2A 1.0 distinguishes the protocol envelope from application-specific structured
data. Its HTTP+JSON binding includes `POST /message:send`; an immediate response
can be a Message instead of creating a Task. Agent Cards declare supported
interfaces, versions, security and capabilities. The current wire uses
`A2A-Version: 1.0`, `application/a2a+json`, camelCase fields and enum role names.
Legacy `kind` discriminators were removed. Task streaming and callback push are
optional mechanisms; neither supplies a global concurrent-work matcher or durable
recipient inbox. The local adapter therefore advertises only its implemented
synchronous coordination-message skill. See specification sections 3, 4, 8, 11
and Appendix A.

Codex documents `PreToolUse` additional context and a visible system warning.
Background hook context reaches the next safe model boundary; it does not start
an idle turn. Hook coverage differs by tool path, so supported hooks cannot
establish complete visibility into every action. Claude's channels can inject
events but remain a preview with authentication, allowlist/organization and
interactive enablement requirements. A transport write is not acknowledgement
that a model processed a message. The baseline uses durable native delivery and
documented hooks, without claiming universal immediate model interruption.

The existing Recollect MCP endpoint is authenticated per request with JSON
responses; its configured-agent roster reports credentials/last use. Workspace
tasks and operations are explicitly account-private. Native sessions already
bind endpoint/device/Brain/host/session/CWD/subagent identity. Tool hooks perform
local capture under short timeouts; network delivery belongs in the plugin worker.
These observations support a separate shared coordination projection and local
inbox/intent queue rather than widening raw-task access or networking in tool hooks.

## Translation and Limits

[ADR 0023](../adr/0023-agent-to-agent-coordination.md) and the
[contract](../contracts/mcp-agent-coordination.md) are implementation authority.
Recollect owns scoped leases, explainable matching, peer-message authorization,
retention and native delivery. Agentgateway can be an optional routing/auth/policy
boundary later; it does not supply these domain rules. No gateway was deployed
or code copied in this research. Exact paths/topics provide a zero-model-call
baseline; semantic equivalence and unobserved future actions remain blind spots.

An attempted official agentgateway tutorial URL ending in
`/docs/standalone/latest/tutorials/agent-to-agent/` returned an internal retrieval
error. It was not treated as absent product support. The protocol research
reported older 0.2.5 example material; current A2A 1.0 authority must control the
adapter. Context7 completed three library resolutions and three documentation queries.
`/websites/a2a-protocol` and `/a2aproject/a2a` returned current 1.0 material.
The exact Rust SDK resolution had no match; official source supplied the missing
evidence. `/websites/agentgateway_dev` returned obsolete configuration shape;
current official source uses binds/listeners/routes and `policies.a2a`. A guessed
A2A security URL and a GitHub directory request failed; the specification and
raw official source succeeded. Failed lookups were not treated as absence.

A subsequent implementation Context7 query to `/a2aproject/a2a` confirmed the
optional `A2A-Version` query parameter and HTTP 400 mapping for unsupported
operation/version errors. It also returned an older `problem+json` error example
conflicting with the current specification's section 11.6. The adapter follows
the current primary specification's `google.rpc.Status` JSON envelope; indexed
examples are useful discovery evidence but do not override that wire authority.

The reviewed gateway release was [v1.6.0](https://github.com/agentgateway/agentgateway/releases/tag/v1.6.0),
released October 2. Its [A2A proxy source](https://github.com/agentgateway/agentgateway/blob/v1.6.0/crates/agentgateway/src/a2a/mod.rs)
handles 1.0 interface rewriting and nested task/message telemetry. The walkthrough
reported 0.2.5, so it is not a current wire template. The
[Apache-2.0 license](https://github.com/agentgateway/agentgateway/blob/v1.6.0/LICENSE)
permits reuse subject to its terms; no code was copied or runtime conformance
proven here.

The official [Rust SDK snapshot](https://github.com/a2aproject/a2a-rs/tree/2b705c3ddcd45df2e2f9f447057acd465581916e)
from October 16 reports core 0.4.2/server 0.5.3/client 0.2.9, Axum 0.8 and
MSRV 1.85 under Apache-2.0. The server subsequently reused its core package as
`a2a = { package = "a2a-lf", version = "=0.4.2" }`, including Message, Part,
Role and send-message request/response types. The focused HTTP integration decoded
the actual Agent Card and response with these official SDK types. This proves the
tested message binding, not the SDK's entire feature set. The
[A2A protocol license](https://github.com/a2aproject/A2A/blob/main/LICENSE) is
also Apache-2.0. Recollect remains responsible for its domain logic and tests.

Native transport research resolved Context7 `/seanmonstar/reqwest`; its request
and response documentation confirmed per-request timeout overrides, asynchronous
response chunks and body-inclusive client timeouts. The implementation retains
the existing reqwest 0.12 dependency. Rust Analyzer initially resolved relevant
symbols; final diagnostics lacked dependency/standard-library indexing, so direct
Cargo compilation supplies the validation evidence rather than those incomplete
diagnostics.

## Follow-up

Validate only the card's advertised binding. Retain synthetic two-account
macOS/Ubuntu proof separately, including SSE delivery, durable pull recovery,
native hook visibility, revocation and proposal/acceptance. A synthetic fixture
does not prove another physical Mac or a production installation is connected.
