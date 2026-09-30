# MCP Coordination, Vault and Private Runners

Status: active

## Purpose

Expose Recollect memory/workspace operations to coding agents and coordinate
their authorized MCP tools per Brain/environment, including Vault credential
delivery, local/central/private execution and safe process lifecycle.

## Governing Sources

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

## Decisions Before Implementation

- Specify definition/connection/profile ownership, approved schema discovery,
  permission evaluation and runtime configuration before the catalogue slice.
- Specify transport authentication, startup/reuse identity, leases, cancellation,
  idle behavior and unknown-completion reconciliation before execution slices.
- Specify Vault auth/paths/credential delivery and rotation with explicit test
  targets; define private-runner trust and supported connector fixtures. Never
  embed credentials in contracts, model arguments or captured output.

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `mcp-direct-auth-repair` | shipped | contract-backed | pack | SWEG credential handoff repaired through Keychain; installed Codex 0.157.1 workspace read and deployed concrete setup instructions verified |
| `mcp-direct-connections` | shipped | contract-backed | pack | Direct HTTP host setup and token, anonymous name/URL form; real Context7 metadata/protocol proof with useful documentation quota-blocked |
| `mcp-catalogue-and-profiles` | shipped | contract-backed | pack | Operator-approved cached definitions, Brain/environment connections, independent profile use/manage/share and desktop catalogue with real RLS/browser proof |
| `mcp-runtime-and-credentials` | shipped | adr-backed, contract-backed | pack | Actual central/paired stdio and HTTP calls, isolated credentials, fenced recovery without effect replay, desktop/native workflow and verified local schema 022 upgrade |
| `mcp-vault-and-private-runners` | shipped | adr-backed, contract-backed | pack | Optional per-connection Vault Proxy delivery, real issuance/renewal/rotation, authenticated private stdio/HTTP/native/desktop execution and preserved normal migration 023 |
| `mcp-memory-and-workspace-tools` | shipped | adr-backed, contract-backed | pack | Stateless paired MCP, native bridge, scoped memory/graph/handover tools and fresh capture defaults proven through real coding hosts |
| `mcp-observation-capture` | shipped | adr-backed, contract-backed | pack | Actual central/local/private receipts become scoped canonical evidence; automatic learning, separate reconciliation, retries, native erasure synchronization, older-state replay and desktop state verified; normal migration 024 preserved |
| `mcp-desktop-setup` | shipped | adr-backed, contract-backed | pack | Guided approved connection setup, profiles/runners and uncertain-call inspection without widening grants |
| `mcp-codex-plugin` | in-progress | adr-backed, contract-backed | pack | Local Codex memory skills in the generated capture plugin and stdio MCP rendering for OpenCode |
| `mcp-plugin-direct-auth` | shipped | adr-backed, contract-backed | pack | Direct plugin/MCP user auth (API token or browser device code, Cognee-style) with no CLI download; companion stays capture-only |
| `mcp-successor-cleanup` | shipped | adr-backed, contract-backed | pack | Device dedupe by normalized name, review-UI removal to autonomous learning log, Ask/Search read-only simplification |
| `desktop-connection-authority` | planned | adr-backed, contract-backed | pack | Agents as the sole incoming-connect home, Connections outbound-MCP only, plain-language placement and tool-group naming with session/context views |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `mcp-direct-auth-repair` | `mcp-direct-connections` |
| `mcp-catalogue-and-profiles` | `platform-durable-work`, `evidence-workspace-scope` |
| `mcp-runtime-and-credentials` | `mcp-catalogue-and-profiles` |
| `mcp-vault-and-private-runners` | `mcp-runtime-and-credentials` |
| `mcp-memory-and-workspace-tools` | `mcp-runtime-and-credentials`, `retrieval-graph-fusion`, `memory-procedures-and-handovers`, `evidence-session-capture` |
| `mcp-observation-capture` | `mcp-vault-and-private-runners`, `mcp-memory-and-workspace-tools` |
| `mcp-desktop-setup` | `platform-desktop-shell` |
| `mcp-codex-plugin` | `mcp-memory-and-workspace-tools`, `evidence-session-capture` |
| `mcp-plugin-direct-auth` | `mcp-memory-and-workspace-tools`, `mcp-runtime-and-credentials` |
| `mcp-direct-connections` | `mcp-desktop-setup`, `mcp-memory-and-workspace-tools` |
| `mcp-successor-cleanup` | `mcp-plugin-direct-auth` |
| `desktop-connection-authority` | `mcp-desktop-setup`, `evidence-desktop-workflows`, `desktop-knowledge-surface` |

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

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the final local image separately from
remaining cross-domain acceptance. The [mcp-desktop-setup closeout](../execution/archive/mcp-desktop-setup.md)
records this owner’s completed checks. The [desktop guide](../../runbooks/desktop-experience.md)
documents current function locations. Other owners keep unfinished criteria active;
prior shipped domain records remain historical evidence rather than redesign proof.

## 2026-09-29 wiring split

[ADR 0017](../../adr/0017-desktop-knowledge-and-ask-experience.md) resolves the
placement conflict between the desktop contract and the managed-experience
amendment: Agents becomes the sole incoming-connection home and Connections owns
outbound Brain-managed MCP only. The `desktop-connection-authority` slice delivers
it. Device authority, profile grants, runner trust, credential handling and the
shipped device dedupe behavior are untouched; this is presentation and route
resolution only.
