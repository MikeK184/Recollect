# MCP Coordination, Vault and Private Runners

Status: active

## Purpose

Expose Recollect memory/workspace operations to coding agents and coordinate
their authorized MCP tools per Brain/environment, including Vault credential
delivery, local/central/private execution and safe process lifecycle.

## Governing Sources

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
| `mcp-ubuntu-private-runner-proof` | shipped | contract-backed | pack | Real Ubuntu filesystem MCP with separate reader caller, permission/boundary denials, offline recovery and fresh OS-store reuse |
| `mcp-equivalent-http-schema-inspection` | shipped | contract-backed | pack | Bounded equivalent Draft 7 declaration conversion during HTTP inspection; strict manifest approval retained |
| `mcp-copy-ready-agent-config` | in-progress | contract-backed | pack | Browser direct-token native config and visible account token management, preserving plugin OS storage |
| `mcp-approved-connector-icons` | shipped | contract-backed | pack | Explicit discovery preserves safe optional connector icons with bounded local cache/fallback |
| `mcp-plugin-session-memory` | shipped | adr-backed, contract-backed | pack | Complete native plugin capture/automatic recall with bundled runtime, OpenCode adapter and optional runner flag |
| `mcp-direct-auth-repair` | shipped | contract-backed | pack | SWEG credential handoff repaired through Keychain; installed Codex 0.157.1 workspace read and deployed concrete setup instructions verified |
| `mcp-direct-connections` | shipped | contract-backed | pack | Direct HTTP host setup and token, anonymous name/URL form; real Context7 metadata/protocol proof with useful documentation quota-blocked |
| `mcp-catalogue-and-profiles` | shipped | contract-backed | pack | Operator-approved cached definitions, Brain/environment connections, independent profile use/manage/share and desktop catalogue with real RLS/browser proof |
| `mcp-runtime-and-credentials` | shipped | adr-backed, contract-backed | pack | Actual central/paired stdio and HTTP calls, isolated credentials, fenced recovery without effect replay, desktop/native workflow and verified local schema 022 upgrade |
| `mcp-vault-and-private-runners` | shipped | adr-backed, contract-backed | pack | Optional per-connection Vault Proxy delivery, real issuance/renewal/rotation, authenticated private stdio/HTTP/native/desktop execution and preserved normal migration 023 |
| `mcp-memory-and-workspace-tools` | shipped | adr-backed, contract-backed | pack | Stateless paired MCP, native bridge, scoped memory/graph/handover tools and fresh capture defaults proven through real coding hosts |
| `mcp-observation-capture` | shipped | adr-backed, contract-backed | pack | Actual central/local/private receipts become scoped canonical evidence; automatic learning, separate reconciliation, retries, native erasure synchronization, older-state replay and desktop state verified; normal migration 024 preserved |
| `mcp-desktop-setup` | shipped | adr-backed, contract-backed | pack | Guided approved connection setup, profiles/runners and uncertain-call inspection without widening grants |
| `mcp-codex-plugin` | shipped | adr-backed, contract-backed | pack | Local Codex memory skills in the generated capture plugin and stdio MCP rendering for OpenCode |
| `mcp-plugin-direct-auth` | shipped | adr-backed, contract-backed | pack | Direct plugin/MCP user auth (API token or browser device code, Cognee-style) with no CLI download; companion stays capture-only |
| `mcp-successor-cleanup` | shipped | adr-backed, contract-backed | pack | Device dedupe by normalized name, review-UI removal to autonomous learning log, Ask/Search read-only simplification |
| `desktop-connection-authority` | shipped | adr-backed, contract-backed | pack | Agents as the sole incoming-connect home, Connections outbound-MCP only, plain-language placement and tool-group naming with session/context views |
| `mcp-brain-agent-roster` | shipped | contract-backed | pack | Per-Brain agent roster (plugin vs MCP-token agents, host kind, brain-scoped last use), Devices out of global nav with hidden pairing deep link, two-integration copy cleanup and the external-MCP-through-Brain proof |
| `agents-surface-refinement` | shipped | contract-backed | pack | Per-Brain roster shows only agents used on this Brain; new global `/agents` page lists every agent with the Brains it is used in and last use there; account-scoped `GET /api/agents` read |

## Slice Dependencies

| Slice ID | Predecessors |
| --- | --- |
| `mcp-ubuntu-private-runner-proof` | `mcp-vault-and-private-runners`, `mcp-memory-and-workspace-tools` |
| `mcp-plugin-session-memory` | `mcp-plugin-direct-auth`, `mcp-codex-plugin`, `evidence-session-capture`, `mcp-vault-and-private-runners` |
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
| `mcp-brain-agent-roster` | `desktop-connection-authority` |
| `agents-surface-refinement` | `mcp-brain-agent-roster` |

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

## 2026-10-01 local closeout

[desktop-connection-authority](../execution/archive/desktop-connection-authority.md) are locally delivered with [dated validation and runtime limits](../../mappings/desktop-continuation-2026-10-01.md).
The user prioritizes normal laptop and larger desktop displays; additional
small-screen and keyboard polish is optional under the [desktop contract](../../contracts/desktop-experience.md#display-and-interaction-priority--2026-10-01).

## Installed-host plugin closeout — 2026-10-01

The [plugin pack](../execution/archive/mcp-codex-plugin.md) is shipped. Actual
Codex skill use, scoped contribution and fresh HTTP/native recall, OpenCode native
stdio workspace.list and the CLI collision refusal pass. User-completed Keychain
approval is followed by successful fresh-process access. The disposable Brain,
credentials and dedicated host server were cleaned up; original inventory and
readiness remain intact. See [dated proof and limits](../../mappings/desktop-final-acceptance-2026-10-01.md).
Marketplace publication, OAuth and OpenCode capture hooks remain separate non-goals.

## Plugin replacement closeout — 2026-10-02

The [complete plugin](../execution/archive/mcp-plugin-session-memory.md) replaces
separate companion setup with native Codex/Claude/OpenCode installation, OS-store
connection, automatic capture/learning/cited recall, scope-safe recovery and
optional `--with-runner` execution. Installed host and bundled checkout-publication
proofs pass. The local API/worker/UI run migration 030 with original inventory
preserved. [Final evidence](../../mappings/plugin-session-memory-2026-10-01.md)
distinguishes Linux/macOS host versions, fresh checks, focused reruns and limits.
This supersedes the earlier OpenCode-capture deferral; marketplace/OAuth remain
non-goals. All slices in this epic are delivered locally and uncommitted.

## 2026-10-02 brain agent roster

The user's 2026-10-02 direction governs the reopened slice: exactly two
integration types (Recollect plugin, direct MCP access token), everything scoped
to a Brain, the workspace-level Devices page out of the main navigation with
agent management inside each Brain's Agents surface grouped by user ("mike has N
agents, active or not"), no `recollect-agent`/companion wording in the UI, and an
end-to-end proof that a connected host can use an external MCP through the
Brain's MCP with central or local placement. Shipped 2026-10-02: migration 031,
issuer markers, the `GET /api/brains/{brain}/agents` roster read, the Devices
nav removal with the pairing deep link preserved, the two-integration copy
cleanup, and the live SWEG host → Brain agent MCP → external Context7 call with
central placement and retained output. The [archived pack](../execution/archive/mcp-brain-agent-roster.md)
and [evidence](../../mappings/brain-agent-roster-2026-10-02.md) record the
checks; contract amendments land in
[device pairing](../../contracts/platform-device-pairing.md),
[plugin direct auth](../../contracts/mcp-plugin-direct-auth.md) and the
[desktop experience](../../contracts/desktop-experience.md). The local-placement
stdio variant remains configuration-only on this host (no packaged runner
helper; live bridge runs without `--with-runner`).

## 2026-10-03 agent surface refinement

The user's 2026-10-03 direction refined the roster: a Brain's roster shows only
agents that have been used on it, and a global `/agents` page (third global-nav
entry) lists every account agent with the Brains it is used in and last use
there. Shipped 2026-10-03: the account-scoped `GET /api/agents` read (owner sees
all account devices, members their own; per-agent brain usage limited to
accessible Brains), the global Agents page, and the used-only per-Brain roster
with an "All account agents" link. The [archived
pack](../execution/archive/agents-surface-refinement.md) and
[evidence](../../mappings/agents-ask-chrome-refinements-2026-10-03.md) record
the checks; the contract amendment lands in
[plugin direct auth](../../contracts/mcp-plugin-direct-auth.md).

## Browser feedback closeout — 2026-10-05

Direct browser config and Access tokens are implemented; native configuration recognition and real HTTP MCP recall pass. The copy-ready pack stays active because the installed OpenCode background service prevented fresh native launch acceptance. Safe optional icon cache/render acceptance is complete.

[Coordinated delivery, validation and limits](../../mappings/desktop-browser-management-2026-10-05.md). Version N/A; work uncommitted, no push or external release.

## Connection inspection follow-up closeout — 2026-10-06

- Planned: Address marked Connections metadata/loading/formatting and Exa Test concerns.
- Shipped: Clear actual badges/read states, formatted descriptions and JSON, structured
  call details and bounded equivalent schema inspection on the ready local stack.
- Not shipped: Broad schema migration, automatic tool execution or new grants.
- New blockers: None for this follow-up; existing native-host work stays separate.
- Docs updated: Contracts, archived packs, indexes, handoff and [current evidence](../../mappings/mcp-inspection-readability-2026-10-06.md).
- Validation: Pure/owned backend fixtures, build/lint/governance, actual CUA read/check/
  expiry/width proof, preserved runtime invariants and independent source/pixel review.
- Version: N/A.
- Commit: Uncommitted.
