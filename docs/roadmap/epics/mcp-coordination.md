# MCP Coordination, Vault and Private Runners

Status: active

## Purpose

Expose Recollect memory/workspace operations to coding agents and coordinate
their authorized MCP tools per Brain/environment, including Vault credential
delivery, local/central/private execution and safe process lifecycle.

## Governing Sources

- [Agent coordination ADR](../../adr/0023-agent-to-agent-coordination.md)
- [Brain-scoped coordination contract](../../contracts/mcp-agent-coordination.md)

- [Brain-managed skills ADR](../../adr/0022-brain-managed-skills.md)
- [Managed skills and instructions contract](../../contracts/mcp-managed-skills.md)
- [Native skill lifetime and refresh contract](../../contracts/mcp-native-skill-reload.md)

- [Plugin-managed memory ADR](../../adr/0018-plugin-managed-agent-memory.md)
- [Plugin session contract](../../contracts/mcp-plugin-session-memory.md)

- [Desktop contract](../../contracts/desktop-experience.md)
- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [Knowledge surface ADR](../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered surface contract](../../contracts/desktop-knowledge-surface.md)
- [Managed observation ADR](../../adr/0012-managed-tool-observations.md)
- [Automatic managed observation capture](../../contracts/mcp-observation-capture.md)
- [Agent memory MCP ADR](../../adr/0010-agent-memory-mcp.md)
- [Direct plugin user auth ADR](../../adr/0015-direct-plugin-user-auth.md)
- [Agent memory/workspace tools](../../contracts/mcp-memory-and-workspace-tools.md)
- [Plugin direct-auth contract](../../contracts/mcp-plugin-direct-auth.md)
- [Vault/private ADR](../../adr/0009-vault-and-private-execution.md)
- [Vault/private contract](../../contracts/mcp-vault-and-private-runners.md)
- [Managed runtime ADR](../../adr/0008-managed-mcp-runtime.md)
- [Runtime and credential contract](../../contracts/mcp-runtime-and-credentials.md)
- [Approved catalogue and profile permissions](../../contracts/mcp-catalogue-and-profiles.md)
- [Vision: MCP coordinator and Vault](../../foundation/vision.md#mcp-coordinator-and-vault-integration)
- [Vision: access and execution placement](../../foundation/vision.md#deployment-and-access-model)
- [Stack: identity, workspace and coordination](../../foundation/techstack.md#identity-workspace-and-mcp-coordination)
- [Engineering principles: process management](../../foundation/engineering-principles.md#separate-tool-selection-from-process-management)
- [Engineering principles: credentials](../../foundation/engineering-principles.md#minimize-captured-source-and-credentials)

## Dependencies and Boundaries

Consume platform identities/grant primitives, paired devices and workspace scope.
Own approved MCP definitions, connections, named execution profiles, independent
use/manage/share grants, catalogue/configuration UI and runtime state.
Definitions/configuration/discovery are distinct from successful live calls.

Own rmcp client/server adapters, local stdio and remote Streamable HTTP routing,
startup locks, leases, compatible instance reuse and the 15-minute useful-idle
shutdown rule. Runner targets, executables/images and credential references stay
outside model-controlled arguments. External hosted processes are not ours to stop.

Own the credential-provider boundary and Vault integration, including delivery,
renewal, rotation and draining. OS-store/environment and anonymous alternatives
can coexist with Vault within one Brain; Vault is optional for each connection.
Delivered Vault support remains first-usable scope. Private runners use authenticated
outbound connections and enforce grants at dispatch and execution. Do not equate
revocation with automatic cancellation or invalidate already-issued credentials
without an explicit supported mechanism.

Memory/workspace tools call existing application handlers, and managed observations
enter evidence capture with actual target, operation, outcome and scope. Reading
production knowledge or retrieving a runbook does not grant production execution.
No generic shell sandbox, unrestricted arbitrary executable registration or
universal catalogue of customer connectors is introduced.

## Slice Map

The [automated memory improvement plan](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/research/automated-memory-improvement-plan-2026-10-07.md)
proposes automatic digest scheduling and delivery of a compact canonical project
brief through the supported plugin/recall path. Memory and retrieval retain primary
ownership; this epic owns host lifecycle, immutable attribution and delivery proof.
The memory-owned [digest pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/memory-automatic-session-digests.md)
and retrieval-owned [brief pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/retrieval-scoped-project-brief.md)
are locally delivered with native ended-task resume and escaped hook bounds.
No installed-binary upgrade or new injection guarantee for third-party direct
MCP clients is implied.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `mcp-agent-coordination` | shipped | adr-backed, contract-backed | archived | Cross-machine presence, explainable work overlap, durable peer messages, SSE/pull/hooks and narrow A2A 1.0 adapter; synthetic macOS/Ubuntu proof passed 15 checks |
| `mcp-native-skill-reload` | shipped | adr-backed, contract-backed | archived | Native Codex/Claude/OpenCode exports, cross-host scope leases, temporary/retained lifetime and actual host reload/activation proof |
| `mcp-managed-skills` | shipped | adr-backed, contract-backed | archived | Brain/environment managed guidance, progressive MCP fetch, plugin cache lifetime and manager UI |
| `mcp-plugin-workflow-repair` | shipped | contract-backed | archived | Supported evidence tools, observed usage, bounded learning recovery and compact visible policy/token UI |
| `mcp-ubuntu-private-runner-proof` | shipped | contract-backed | archived | Real Ubuntu filesystem MCP with separate reader caller, permission/boundary denials, offline recovery and fresh OS-store reuse |
| `mcp-equivalent-http-schema-inspection` | shipped | contract-backed | archived | Bounded equivalent Draft 7 declaration conversion during HTTP inspection; strict manifest approval retained |
| `mcp-copy-ready-agent-config` | in-progress | contract-backed | pack | Browser direct-token native config and visible account token management, preserving plugin OS storage |
| `mcp-approved-connector-icons` | shipped | contract-backed | archived | Explicit discovery preserves safe optional connector icons with bounded local cache/fallback |
| `mcp-plugin-session-memory` | shipped | adr-backed, contract-backed | archived | Complete native plugin capture/automatic recall with bundled runtime, OpenCode adapter and optional runner flag |
| `mcp-direct-auth-repair` | shipped | contract-backed | archived | test-Brain credential handoff repaired through Keychain; installed Codex 0.157.1 workspace read and deployed concrete setup instructions verified |
| `mcp-direct-connections` | shipped | contract-backed | archived | Direct HTTP host setup and token, anonymous name/URL form; real Context7 metadata/protocol proof with useful documentation quota-blocked |
| `mcp-catalogue-and-profiles` | shipped | contract-backed | archived | Operator-approved cached definitions, Brain/environment connections, independent profile use/manage/share and desktop catalogue with real RLS/browser proof |
| `mcp-runtime-and-credentials` | shipped | adr-backed, contract-backed | archived | Actual central/paired stdio and HTTP calls, isolated credentials, fenced recovery without effect replay, desktop/native workflow and verified local schema 022 upgrade |
| `mcp-vault-and-private-runners` | shipped | adr-backed, contract-backed | archived | Optional per-connection Vault Proxy delivery, real issuance/renewal/rotation, authenticated private stdio/HTTP/native/desktop execution and preserved normal migration 023 |
| `mcp-memory-and-workspace-tools` | shipped | adr-backed, contract-backed | archived | Stateless paired MCP, native bridge, scoped memory/graph/handover tools and fresh capture defaults proven through real coding hosts |
| `mcp-observation-capture` | shipped | adr-backed, contract-backed | archived | Actual central/local/private receipts become scoped canonical evidence; automatic learning, separate reconciliation, retries, native erasure synchronization, older-state replay and desktop state verified; normal migration 024 preserved |
| `mcp-desktop-setup` | shipped | adr-backed, contract-backed | archived | Guided approved connection setup, profiles/runners and uncertain-call inspection without widening grants |
| `mcp-codex-plugin` | shipped | adr-backed, contract-backed | archived | Local Codex memory skills in the generated capture plugin and stdio MCP rendering for OpenCode |
| `mcp-plugin-direct-auth` | shipped | adr-backed, contract-backed | archived | Direct plugin/MCP user auth (API token or browser device code, Cognee-style) with no CLI download; companion stays capture-only |
| `mcp-successor-cleanup` | shipped | adr-backed, contract-backed | archived | Device dedupe by normalized name, review-UI removal to autonomous learning log, Ask/Search read-only simplification |
| `desktop-connection-authority` | shipped | adr-backed, contract-backed | archived | Agents as the sole incoming-connect home, Connections outbound-MCP only, plain-language placement and tool-group naming with session/context views |
| `mcp-brain-agent-roster` | shipped | contract-backed | archived | Per-Brain agent roster (plugin vs MCP-token agents, host kind, brain-scoped last use), Devices out of global nav with hidden pairing deep link, two-integration copy cleanup and the external-MCP-through-Brain proof |
| `agents-surface-refinement` | shipped | contract-backed | archived | Per-Brain roster shows only agents used on this Brain; new global `/agents` page lists every agent with the Brains it is used in and last use there; account-scoped `GET /api/agents` read |
| `mcp-credential-resolution-freshness` | shipped | contract-backed | inline: remove the pre-read credential reuse so each operation resolves again as the contract requires, keep the after-read same-object return, drop an upstream-rejected resolution without retrying the call; unit tests plus the existing platform permission-loosening proof without `invalidate()` | Rotated OS-store/reference-file values and loosened key files take effect on the next call, unchanged values keep their generation, and an upstream 401/403 drops that connection's cached resolution; 3 unit tests (rotation, rejection, real HTTP 401/403 with no retry) and the `mcp::` platform subset unchanged (30 pass, the same 13 unrelated failures as before) |
| `agents-roster-capture-reads` | shipped | contract-backed | inline: roster capture use reads only each binding's newest unexpired accepted event (partial index `capture_binding_latest`, migration 073, replaced `recollect_brain_agent_usage` body and per-binding host fallback) instead of every capture event under per-row RLS and retention checks; roster fields, ordering, visibility and polling unchanged; platform test `agent_rosters_read_only_the_newest_capture_of_each_binding` (20k events, removed and retention-expired newer events ignored, each roster under 2 s through the index) fails on the previous code and passes, existing roster tests pass, browser proof on a disposable 20k-event database | My agents stayed on "Loading agents…" for 6–7 s on the 2026-10-19 local stack (`GET /api/agents`, about 10.8k capture events); on a copy of that data the roster SQL fell from 5.4 s to 0.11 s (account) and 7.3 s to 0.08 s (busiest Brain) with identical rows, and on main plus this change the browser showed My agents in 146 ms (old account SQL 9.1 s, new 9 ms on that 20k-event fixture); the running stack still needs a rebuild |

## Completion Criteria

- Discovery lists authorized capabilities without starting dormant backends;
  knowledge access never implies execution, including for administrators.
- Actual local, central and private calls select the approved target/credential
  configuration. Model argument substitution and revoked new calls are denied.
- Concurrent startup starts one compatible instance; active/long operations
  survive other sessions ending and idle timers. Only owned processes are stopped.
- Vault credential rotation and runner interruption exercise renewal/draining
  behavior without leaking values to model context, logs or retained captures.
- Unknown side-effect completion is reconciled instead of blindly retried.
  Captured success/failure preserves what was actually observed.
- Codex/Claude memory/workspace integration refreshes applicable context after
  scope changes without relabeling in-flight tasks or inventing missing events.

## Current status

[Managed skills](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-managed-skills.md) are locally delivered:
administrator-authored Brain/environment guidance, scoped progressive HTTP/MCP
fetch, plugin metadata/local-copy lifecycle and the Agents Skills management tab.
Focused database, real MCP, protocol/plugin, mocked browser and documentation
proof passed; see the [first evidence](../../mappings/managed-skills-host-consumption-2026-10-18.md).
The [native successor](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-native-skill-reload.md) adds owned
Codex/Claude/OpenCode exports, cross-host scope leases, temporary/retained lifetime
and actual same-process native activation/reload proof for all three hosts.
Its [evidence](../../mappings/native-skill-reload-host-proof-2026-10-18.md) records
94 passing agent tests, nine JavaScript adapter tests and isolated installed-host
results. Updating the user's installed plugin or normal deployment remains separate.

The [agent coordination pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/mcp-agent-coordination.md) is
locally delivered. Its shared session projection preserves private workspace
tasks, finds topic overlap before edits and planned-file overlap, and supports
durable peer negotiation through SSE/pull, MCP, native hooks/CLI and A2A 1.0.
The [dated proof](../../mappings/agent-coordination-proof-2026-10-18.md) records
15 passing macOS/Ubuntu checks and zero canonical model requests. Installed-model
consumption, a second physical Mac and production deployment remain separate claims.

Managed connections, independent Use/permission checks, Vault/private runners and plugin source workflows are delivered. The direct-token pack retains final native-host launch acceptance after an OpenCode service timeout; fresh Claude launch and rebuilt-plugin acceptance remain separate from successful HTTP/configuration proof. Preserve the recorded host-specific limits when continuing.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/mcp-coordination.md).
Dated test/deployment observations are not a current runtime health claim.
