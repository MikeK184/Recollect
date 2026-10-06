# Browser management review and implementation plan

Status: approved historical investigation; implemented under successor packs

Date: 2026-10-05

The user subsequently approved implementation of all ten points and the
Connections-style cards with adjacent green/red Use/Manage/Share icons. See the
[current delivery and remaining native-host validation](../mappings/desktop-browser-management-2026-10-05.md).
The findings, before measurements and proposal language below describe the
original read-only investigation, not current runtime state.

## Scope and evidence

The user requested a parent-agent and independent-agent investigation of ten
browser comments, a proper plan, and a tool-group design image. This record
addresses all ten. It is a proposal, not new implementation authority or a
shipment claim. Existing uncommitted work and credentials remain untouched.

Evidence includes the supplied marked screenshots, read-only live browser
inspection at 1897×1314, current source, accepted contracts, CodeGraph status and
exact-symbol navigation, an independent source/design review, and official
interface documentation fetched on this date. No tokens were created, revealed,
revoked or sent to external services. No paid model calls, tool execution,
application rebuild or deployment was performed for this investigation.

Live measurements at the same viewport and CSS zoom of 1:

| Route | Sidebar width | Navigation font | Page heading | Description |
| --- | --- | --- | --- | --- |
| Brains | 248px | 14px | 40px | Not measured |
| Connectors | 264px | 16px | 50px | 22px |
| Brain Agents | 264px | 16px | 50px | Not measured |
| Brain Settings | 264px | 16px | 42px | Not measured |

The agent roster, raw activity cards, automatic private-context entry point,
read-only installed-model cards, and optional recommended-default section are
still present in the running UI. Some connection setup/editor work has changed
since the supplied screenshots; current source determines the remaining work.

## Governing route and ownership

- [Desktop contract](../contracts/desktop-experience.md), especially shared
  typography and the October 5 management amendments; owning
  [desktop epic](../roadmap/epics/desktop-visual-experience.md).
- [Device pairing](../contracts/platform-device-pairing.md),
  [plugin direct auth](../contracts/mcp-plugin-direct-auth.md), and
  [plugin-managed session memory](../contracts/mcp-plugin-session-memory.md).
  Account/device authority belongs to the
  [platform epic](../roadmap/epics/product-platform.md); host integration belongs
  to [MCP coordination](../roadmap/epics/mcp-coordination.md).
- [MCP catalogue and grants](../contracts/mcp-catalogue-and-profiles.md) and
  [runtime/credentials](../contracts/mcp-runtime-and-credentials.md).
- [Provider policy](../contracts/memory-provider-policy-and-learning.md),
  [managed experience](../contracts/memory-managed-experience.md), and
  [semantic generations](../contracts/retrieval-semantic.md). Model policy belongs
  to [memory lifecycle](../roadmap/epics/memory-lifecycle.md); representation
  compatibility belongs to [hybrid retrieval](../roadmap/epics/hybrid-retrieval.md).
- Accepted vision, technology stack and engineering principles preserve the
  independent Rust backend, one policy boundary, agent-managed scope, and
  separate knowledge, transmission and execution permissions.

Previous desktop packs remain historical closeouts. Add successor slices to
their actual owners and create decision-complete execution packs before
significant implementation. Amend the affected contracts for the user's changed
requirements; label any still-unresolved dependency `needs-contract` before its
dependent implementation. Independent visual corrections can proceed first.

## Ten comments: findings, final direction and proof

### 1. Connectors should use the normal page scale

**Finding:** `web/src/features/management.css` uses route-sensitive `:has(...)`
selectors to enlarge the workspace grid, navigation, icons, spacing and page
typography. This is an application styling regression, not browser zoom.

**Plan:** Remove route-dependent shell sizing. Reuse the shared sidebar, topbar,
navigation, page-header and row tokens used by Brains/My agents. Target the
existing 248px desktop sidebar, 14px navigation/controls and 36–40px Newsreader
headings, with ordinary concise descriptions. Preserve cream/ink/sage styling
and the existing connector functions.

**Proof:** Compare computed shell geometry/fonts across all six management/global
routes at one fixed viewport/zoom, then inspect actual pixels at 1440×900,
1920×1080 and the user's 1897×1314. Navigation must not resize when routes change.

### 2. Diagnostics should always be available at the top right

**Finding:** `web/src/App.tsx` places Team and `OperationsDialog` in the account
utilities menu. Healthy-service CSS hides the separate health group.

**Plan:** Put an accessible Runtime diagnostics icon at the far right of the
persistent topbar on every route for the installation owner. Keep it visible
when healthy; show meaningful status through its tooltip or a small exception
indicator. Keep Team in account utilities. Reuse the existing diagnostic dialog
and retain current authority. Load/poll its feeds only while open.

**Proof:** Icon is reachable on global and Brain routes, healthy and failing
states; closed diagnostics stop their history polling. Non-owners do not gain
operator access. It does not overlap the breadcrumb or other shell controls.

### 3. Brain Agents should also use the normal scale

**Finding:** The same shell rules enlarge Brain Agents; later CSS also increases
roster text. Global My agents and Brain Agents need consistent presentation.

**Plan:** Apply the shared header/control/row scale from point 1 to both rosters.
Keep user grouping in a Brain and used-Brain grouping in My agents; use compact
host-icon rows with name, integration, credential state and last observed use.

**Proof:** Match equivalent fonts and control geometry between the two routes;
long names and multiple users remain readable without oversized blank space.

### 4. Direct token in copied config, and a discoverable credential list

**Finding:** `web/src/McpAgentSetup.tsx` generates `bearer_token_env_var`,
environment placeholders, or a macOS Keychain helper. `DevicesPanel.tsx` lists
safe credential metadata through My agents → Manage access. The raw secret is
only available during pairing; device-list responses exclude it.

**Plan:** Keep plugin connection as the primary automatic-memory option. For
Direct MCP, create a labelled credential and generate one complete TOML/JSON
configuration containing its literal Authorization header. Remove the required
export/read/Keychain step from this path. Keep advanced storage options secondary.
The generated secret remains transient until the user copies/downloads it into
their private, untracked host configuration. Mask the on-screen header by
default, offer an explicit reveal, and copy the exact valid config with the
actual token. Never retain it in browser storage, logs, URL, analytics or examples.

Codex's intended generated shape, using a placeholder in this proposal:

```toml
[mcp_servers.recollect]
url = "http://127.0.0.1:8787/api/brains/<brain-id>/mcp/agent"
http_headers = { Authorization = "Bearer <new-token>" }
```

Generate native Claude/OpenCode JSON as well, validated against the supported
host's schema. Official OpenCode documentation currently shows `mcp.<name>`;
Recollect's generator uses `mcp.servers.<name>` and the installed binary reports
v2.0.22. Resolve this version/schema discrepancy before asserting compatibility;
do not blindly replace a locally supported v2 shape with an online example.

Expose an **Access tokens** entry beside Connect agent / in My agents, backed by
the existing own-account list. Show name, host/integration, creation, expiry,
last use and revoke/history controls, including tokens with no Brain activity.
Secrets remain one-time; later replacement creates a fresh credential rather
than revealing stored bearer values. Explain expiry when created. A token acts
as its user and follows their Brain rights; the endpoint's Brain URL does not
make the credential Brain-bound. Revocation affects that credential across Brains.

Direct setup currently names credentials only by host/Brain, and approval can
reuse a same-name active device, replacing its token. New-token creation must
mint a distinct record/name and must not silently invalidate a working connection.
Retain retry/cancel handling and a real authenticated memory-read check; copied
configuration is not proof of connection. Amend the direct-MCP setup contract
for this explicit user preference while preserving native plugin OS storage.

**Proof:** TOML/JSON parsing and exact copying with a temporary synthetic token;
no mandatory environment variable; no secret leakage after close/navigation;
new-token creation preserves an existing working token; list visibility,
foreign-account denial, 30-day expiry and revocation; actual supported-host read
with an isolated disposable credential during implementation validation.

### 5. Simpler agent activity without repeated subtext

**Finding:** `AgentsPage.tsx` opens a large dimming drawer; `CapturePanel.tsx`
repeats actor, agent, host, reported time, processing prose and technical details
inside separate cards.

**Plan:** Select an agent in the roster and show an adjacent compact inspector,
using the same pattern as Connections. Header: host icon, name, credential state
and last observed use. Body: date-grouped activity rows with event kind/title,
time and one outcome. Put evidence and technical detail behind an individual
row expansion. Remove repeated owner/agent identity and general tutorial text.
Keep an actionable empty/error state and a concise partial-coverage marker only
when relevant. Use a lightweight drawer fallback when space is insufficient.

Do not rename an enabled credential to Online, invent activity summaries not
supported by records, or merge receipts into a complete session without a real
host session identity. Private task/context/checkout records remain private;
permitted captured evidence retains its existing Brain read grants.

**Proof:** No repeated identity blocks; selected agent stays visible; evidence
opens correctly; loading/empty/failure/role-loss states clear stale protected
content. Captured, learned and observed-use states remain distinct.

### 6. Hide automatically managed private context and local folders

**Finding:** `WorkspacePanel.tsx` says management is automatic while Agents still
exposes Your private context and Work contexts / Local folders tabs.

**Plan:** Remove their ordinary navigation, helper links and drawers from Agents
and activity. Retain discovery, scope records and agent APIs. Old
`?tab=contexts` links normalize to Agents without opening an automatic-context
panel. Keep exceptional debugging in an appropriately authorized diagnostic
surface if required; do not move private paths into other users' activity.

**Proof:** Entry points disappear; plugin discovery/scope still works; legacy
links remain usable; other users cannot see private contexts or local folders.

### 7. Edit the existing connection preview in place; show real icons

**Finding:** `McpPanel.tsx` has the desired inspector, but edit launches
`McpConnectionDialog.tsx`. Icons are always the generic Box. Discovery currently
returns tool descriptors rather than preserved server-icon metadata. The
catalogue contract explicitly prohibits fetching icons today.

**Plan:** Turn the same inspector into the editor: name, URL, approved connector,
environment, placement, credential selection and configuration values become
inputs in their existing rows. Identity, tabs, section titles and column widths
remain stable. Show Cancel/Save in a reserved inspector footer. Retain masked
secret controls, approved schemas, stale-revision detection and partial
credential-provisioning retry behavior. Do not perform a test automatically on
Save; keep Test connection explicit and distinct from tool execution.

For icons, preserve optional MCP server icon metadata during explicit discovery
and prefer the approved connector's icon. Amend the no-fetch contract narrowly:
cache bounded validated images locally, never send credentials/cookies to icon
URLs, reject unsafe targets/redirects and active content, and use the stable
generic icon if metadata is absent or fetching fails. Include private-target and
resource-bound rules before implementation. Do not invent a vendor logo or assume
every server publishes one. Tool-specific icons can remain a later extension.

**Proof:** Read/edit/Cancel/Save keep layout and values; URL/environment changes
invalidate old test evidence; revision conflict retains the draft; failed secret
provision can retry without creating duplicate resources; icon absence/failure
does not block setup or create a request to an unapproved destination.

### 8. Proper Markdown and highlighted JSON/TOML/YAML/code

**Finding:** The repo already bundles lowlight 3.3.0, react-markdown and remark-gfm.
The independent agent verified JSON/YAML/TOML/INI/Bash grammars are registered.
`MarkdownRenderer.tsx` has safe token spans and exact copy, but connection tool
descriptions and schemas still use plain Text/Code. Discovery also collapses
whitespace, so stored descriptions may already have lost lists and code fences.

**Plan:** Export/reuse one bounded, language-labelled CodeBlock and safe Markdown
renderer throughout setup/configuration, tool descriptions, schemas, requests
and results. Pretty-print known JSON objects; preserve exact source bytes for
copying TOML/YAML/commands. Keep code safely readable for unknown/oversized inputs.
Preserve bounded Markdown/newlines at discovery rather than flattening them.
Existing flattened descriptions remain plain until an explicit metadata refresh;
do not pretend a renderer can recover their lost structure. Keep remote text inert
and suppress embedded remote images/HTML. Serve the existing library and light
styles in the application bundle; no runtime Prism/CDN download is necessary.

**Proof:** Highlight each requested language, nested schemas and fenced examples;
copy equals source; malicious Markdown stays inert; long input stays bounded;
fresh metadata keeps line breaks and legacy flattened metadata remains readable.

### 9. Tool groups with inline MCP and people editing

**Finding:** Tool groups use broad repeated cards and separate modal editors.
Their Use / Manage / Share grants are independent and must remain so.

**Plan:** Give each tool group its own card using the existing Connections
styling: dark icon tile, Newsreader title, sage identity header, light paper body,
subtle border and compact actions. Show the group name once, with its enabled
state, environment and MCP/tool count. MCP connections and people/groups sit in
parallel card sections at desktop widths and stack on smaller screens. Each
person has separate Use / Manage / Share controls. Edit turns the existing
values into controls in the same card; Add MCP and Add person/group stay in their
sections; Cancel/Save stay in the reserved footer. Put inheritance/effective-access
detail behind a compact disclosure, with visible indicators where a checkbox
alone would misrepresent it. Remove repeated permission tutorials. One existing
group uses one generous card; do not invent groups to fill the screen. Multiple
groups can stack or use two columns when the available width fits the controls.

The user rejected the earlier left-list/right-inspector concept and requested
the Connections card appearance. This card composition supersedes that proposal.

Connection membership/name/settings require Manage. Grants require Share; either
may be granted independently. Disable only the fields the user cannot change,
and show effective inherited access separately from removable direct grants.
Preserve environment restrictions and failed/stale-save drafts. The existing
profile/grant mutations are separate transactions: if one Save spans both,
either add a genuinely atomic command or show partial results and retry only
unfinished mutations. Never imply all edits saved when only one mutation succeeded.

**Mockup:** [Revised card design image](../../output/browser-review-2026-10-05/tool-groups-cards.jpg).
The interactive concept uses the current `test` group and Context7 connection
identity with illustrative grants and an optional example teammate; it is not an
access audit. It demonstrates editable values, add/remove and separate grants
inside the card. No real configuration or grant changes through this concept.
An independent agent reviewed the rendered revision against Connections and found
no concrete visual blocker; this is visual review, not user acceptance.

**Proof:** Card identity and sections survive editing; Add/remove MCP and users/groups,
independent authority, inherited grants, Cancel, partial failure, stale revision
and environment constraints work without lost fields or misleading success.

### 10. Inline AI permissions, selectable priced models, creation defaults

**Finding:** Purpose badges and content checkbox-like divs are noninteractive;
editing opens a modal. Installed-model fields are read-only. The backend rejects
models not equal to installation configuration, and semantic storage is currently
`vector(3072)`. Browser Brain creation already applies managed defaults atomically.

**Plan:** Preserve the AI page layout the user likes. Let Edit enable the existing
purpose rows, automatic-memory controls and content checkboxes in place, with
Cancel/Save using the existing policy revision and idempotency contract. Make
dependent policy changes explicit: disabling learning/embedding also stops the
corresponding automatic flag; privacy capture permission stays independent.
Remove Optional recommended defaults from Settings. Continue to apply the agreed
defaults once when a Brain is created; never overwrite customized existing Brains.

Rename Installed models to Models and make text/embedding selection editable in
those same cards. Fetch account-available IDs server-side from OpenAI `/v1/models`
using the existing credential. Join IDs with a maintained, dated compatibility
catalogue for the actual Recollect consumers and official pricing metadata.
Model-list responses contain neither prices nor sufficient capability information.
Do not scrape prices on every Settings visit. Show input/cached-input/output USD
per million tokens for text and input price/dimensions for embeddings, including
the applicable tier/region/long-context qualifications. Unknown prices remain
Unknown; stale metadata includes its date. Failed refresh retains the current
selection without presenting unavailable models as verified.

Amend provider policy so admins select an approved supported pair per Brain,
with no arbitrary endpoint/provider or automatic fallback. Validate structured
output, extraction, answering, reranking and request-parameter compatibility;
the current Luna-specific request shaping is insufficient for arbitrary IDs.
List availability is not proof the selected model can complete those operations.

Embedding selection must include dimension support and storage changes. Add
model/dimension-safe representation storage and new generations; do not simply
put a smaller vector in the current fixed column. Saving a changed embedding
model/dimensions uses an explicit **Save and rebuild** action that retires
incompatible vectors and queues a new generation through existing standing
permissions/budgets. Show the rebuild/cost consequence only while that selection
has changed. If current permissions block rebuilding, preserve a visible blocked
state without starting provider calls. Show rebuilding
coverage; exact/lexical access continues under its normal policy. Never mix
models/generations or silently send previously disallowed content to reindex.
Record this Save-and-rebuild trigger and cost consequence in the execution contract.

**Proof:** Inline control changes persist/revert and reject stale policy writes;
readers cannot edit; purpose dependencies work; new Brain defaults are applied
once and existing custom policies survive; unavailable/unsupported models are
excluded; prices have provenance and unknown states; provider failures expose safe
errors; embeddings prove dimensions, generation isolation, policy/erasure fences,
restart/reindex behavior and truthful coverage.

## Delivery order and proposed successor boundaries

| Order | Proposed slice | Owner | Comments | Dependency |
| --- | --- | --- | --- | --- |
| 1 | `desktop-consistent-management-scale` | Desktop | 1, 2, 3, 6, defaults UI from 10 | Shared desktop contract amendment; no backend schema work |
| 2 | `desktop-readable-management-content` | Desktop | 5, 8 | Safe renderer reuse; bounded Markdown-preserving discovery clarification |
| 3 | `mcp-copy-ready-agent-config` | MCP coordination with platform | 4 | Native config schema proof and direct-token lifecycle contract |
| 4 | `desktop-inline-management-editors` | Desktop with MCP coordination | 7 editing, 9, permissions from 10 | Existing revision/grant rules; resolve combined-save semantics |
| 5 | `mcp-approved-connector-icons` | MCP coordination | 7 icons | Approved metadata/image-fetch/storage contract |
| 6 | `brain-model-catalogue-and-selection` | Memory lifecycle with hybrid retrieval | Model/embedding part of 10 | Capability/pricing catalogue, gateway and dimension/generation contract |

These names are proposed boundaries, not registered or shipped slices. Before
implementation, register the agreed slices, reconcile contract amendments, and
write the owning packs. Parts 1–4 do not wait for optional icon/model integration.
No new automatic approval gate is implied; the remaining work is to make the
dependent behavior explicit and reviewable in its governing sources.

## Validation and closeout plan

1. Capture existing authorized UI/inventory before changes; never use real
   customer secrets or alter another account's credentials in proof.
2. Run focused UI cases for each meaningful edit/error/grant state and shared
   cross-route geometry. Inspect actual screenshots at the three desktop sizes.
3. Use isolated server fixtures for credential, catalogue, stale revision,
   partial saves, model availability and embedding generation/fence failures.
   Run fixed synthetic provider checks only under their approved cost/transmission
   authority; never send a customer corpus merely to prove connection.
4. Require a second agent to inspect final pixels and relevant interactions;
   compare the result with all ten comments and the retained design concept.
5. Run affected Rust/frontend checks, `./scripts/validate.sh`, whitespace checks,
   and CodeGraph sync after source changes. Rebuild the local stack and verify
   readiness only as part of the implementation closeout.
6. Update the affected contracts, evidence, packs, owning epics and indexes
   together. Record shipped/not shipped and unresolved items accurately.

## External evidence and limits

- [Codex MCP static headers](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
  and [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
  document literal HTTP headers. This establishes supported configuration shape,
  not a tested Recollect credential or current host read.
- [Claude Code MCP](https://code.claude.com/docs/en/mcp) documents HTTP bearer
  headers and JSON MCP configuration.
- [OpenCode MCP](https://opencode.ai/docs/mcp-servers/) documents remote literal
  headers, with the version/schema discrepancy noted in point 4.
- [MCP implementation icons](https://modelcontextprotocol.io/specification/2025-11-25/schema#implementation)
  are optional metadata. Recollect has not preserved/fetched them in this path.
- [OpenAI model list](https://developers.openai.com/api/reference/resources/models/methods/list)
  provides identifiers/basic availability;
  [official pricing](https://developers.openai.com/api/docs/pricing) and
  [embedding API](https://developers.openai.com/api/reference/resources/embeddings/methods/create)
  provide separate price/dimension evidence. No account model-list call was made.
- Context7 resolve/query for `/highlightjs/highlight.js` succeeded; primary
  [language/alias documentation](https://github.com/highlightjs/highlight.js/blob/main/docs/api.rst)
  supports deliberate language registration. The existing installed lowlight
  registry was also checked read-only by the independent agent.
- Referenced Cognee material was an example of literal-header ergonomics. Its
  supplied credential was neither retained in this record nor used. Recollect
  keeps its own API, identity and revocation model.

## Investigation closeout

- Done: all ten comments reconciled with current source, governing docs and
  available live evidence; independent review; tool-group concept and plan.
- Verified: cross-route live scale measurements and safe read-only inspection;
  official config/model/icon interfaces; existing highlighting registry.
- Plan validation: `./scripts/validate.sh` passed governance lint and all 32
  governance tests; whitespace check passed. Independent review confirmed all
  ten points and the rendered image, with capture-sharing wording corrected.
- Card revision: read and edit screenshots were independently reviewed against
  Connections. Local concept checks covered independent grants, Save, Cancel,
  MCP removal/reselection and adding an example person. At a 1024px viewport,
  the card retained its 752×361px geometry in both read and edit modes. At 320px,
  the sections stacked with no horizontal overflow. Governance lint and all 32
  tests passed again. These checks validate the concept, not application behavior.
- Still open: every application/backend change and its implementation proof.
- Version: N/A; commit: uncommitted. No release, push or deployment.
