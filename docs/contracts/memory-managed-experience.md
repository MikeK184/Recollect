# Managed autonomous memory experience

Status: accepted

## Source

The user's 2026-09-28 correction explicitly puts the agent memory loop ahead of
manual UI configuration. This refines [ADR 0006](../adr/0006-autonomous-memory.md)
and the [vision](../foundation/vision.md): agents choose scope, retrieval depth
and useful contributions; people connect the system once and may inspect or
correct it. Atlas's zero-LLM capture and decay/reinforcement patterns support
decoupled capture and the separation of retirement, truth and physical erasure.

## Contract

### One managed setup

Provide one server-owned preset, `managed`, for supported sanitized capture,
autonomous learning/reconciliation, automatic embeddings and grounded answers.
Its model purposes are extraction, synthesis, embedding and answering. All
existing supported content classes are permitted. Models and credentials come
from the installation. The preset sets a daily accounting ceiling of 1,000,000
tokens, two concurrent calls, a 32 KiB input safety bound and 4,096 output tokens.
These are bounded engineering defaults, not measured optimal budgets or a cost
promise. Input bytes remain a transport/admission bound, not a token count.
Agents retain the existing bounded retrieval/context parameters through MCP.

The browser's new-Brain flow requests this preset and states its provider use.
Legacy API callers that omit that request retain their prior behavior. A missing
provider credential leaves capture available and explains why enrichment cannot
run; adding a credential does not substitute another provider or model.
Existing Brains are never bulk-converted by a deployment. Their administrator can
adopt the preset once from Ask or Settings; no individual numeric fields or
purpose lists are required. This is explicit standing authorization to process
this Brain's eligible retained sources with its installed provider.

`GET /api/brains/{brain}/automation` reports current model, capture, retention
and installed-provider configuration. `PUT` accepts the observed model and
capture change IDs and atomically applies the preset through shared persistence
helpers, audit and outbox. It is browser-admin-only, rejects archived Brains,
checks stale revisions, and supports the existing idempotency protocol. No
external call occurs in this transaction. Capture exclusions are preserved;
supported event kinds and managed-tool capture become enabled with a 32 KiB
event bound. Existing retention/storage policies and durable corrections remain
unchanged. Fresh Brains inherit the accepted 30-day raw-session/tool defaults;
durable knowledge is not erased just because it is old. Existing maintenance
workers retain responsibility for revision, expiry, erasure and recovery fences.

### Human and agent experience

The latest [unified Privacy form](desktop-experience.md#unified-privacy-form-follow-up--2026-10-06)
uses title separators and a single Edit/Save/Cancel group for the existing fields.
It preserves independent canonical commands and partial-save receipts, without
the redundant AI footer link or Activity/backup explanation paragraphs.

The [visible settings follow-up](desktop-experience.md#visible-management-settings-follow-up--2026-10-06)
supersedes collapsed Privacy/AI field presentation below. Automatic processing,
capture limits and activity/backup settings use visible inline sections; empty
privacy exclusions are omitted from the normal read view. Semantic coverage is
optional diagnostic information, not a required human-review task. Existing
standing permissions and bounded automatic recovery remain authoritative.

Amended 2026-09-29 by [ADR 0017](../adr/0017-desktop-knowledge-and-ask-experience.md)
and the [tiered surface amendment](desktop-knowledge-surface.md): the agent
connection cards named below no longer live inside Connections. Agents is the
sole home for connecting a coding tool, and Connections owns outbound
Brain-managed MCP only. The managed preset, progressive disclosure and owner
connector approval behavior in the rest of this section are unchanged.

The primary Brain sidebar shows Ask, Memory, Sources, Graph and Connections.
Repositories, Agents, Activity and Settings remain reachable in a collapsed
workspace group that opens on their routes. Connections starts with supported
agent connection cards, with MCP servers and tool groups on secondary tabs.
The host wizard presents pairing, automatic capture setup and verification in
three short steps. Existing URLs and authorized capabilities remain supported.

Settings primarily reports automation and lifecycle status. Detailed model,
capture, storage and retention overrides are collapsed under advanced controls.
Capture setup connects the supported host/companion; it does not claim coverage
before events arrive. Ask offers a question first; optional search controls are
collapsed, and numeric retrieval budgets are not ordinary form fields. Simple
manual contributions are source notes processed through the existing evidence
pipeline; structured claim forms remain an explicit advanced override. A note
is evidence of what was written, not automatic operational verification.

### MCP setup

Connections must have an actionable empty state. Installation owners can import
an approved connector manifest through the authenticated browser using the same
validation as the CLI. Other Brain admins see the exact operator prerequisite.
`POST /api/mcp/definitions` is owner-browser-only and records an installation
audit; it accepts only a new stable definition key (existing keys use the CLI
update path). No discovery, download, installation or execution occurs during
approval. The current non-secret schemas, command/placement restrictions and
capacity bounds remain. Device/model clients cannot approve implementations.
This narrow owner setup path amends the CLI-only approval entry in the
[catalogue contract](mcp-catalogue-and-profiles.md), not execution permission.
Profiles are explained as named tool groups with separate Use/Manage/Share rights.
Connection and profile creation continue through existing shared handlers.

## Acceptance

Prove new-Brain managed setup and existing-Brain atomic adoption, stale/replayed
requests, reader/contributor/device denial, archived denial, capture exclusions
and retention preservation, provider absence, and no cross-Brain changes. Prove
owner-only connector import with malformed, duplicate and no-execution controls.
Browser checks cover a clear Ask activation path, hidden tuning, optional source
notes, capture setup and the empty connector catalogue. Reuse the existing
autonomous closed-loop proof; do not claim a new universal memory benchmark.

## Explicit Deferrals

No arbitrary host telemetry, connector marketplace, new provider, new ranking
algorithm, adaptive budget controller or automatic tool execution is introduced.
The other open desktop acceptance packs remain required independently.
