# Approved management concepts

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

Subsequent correction, 2026-10-05: the user rejected this delivery’s visual
match after the before/vision/after comparison. Visual acceptance below is
historical and superseded by that rejection. The
[archived fidelity pack](desktop-management-vision-fidelity.md) owns
correction and independent reinspection; prior functional proof remains history.

## Summary

- Goal: Deliver all six approved Connections, setup, global Connectors, Privacy,
  AI permissions and Agents concepts with truthful authorized data.
- Non-goals: New marketplace, automatic connector execution, a new general vault,
  external deployment, changes to unrelated dirty work or research checkouts.
- Delivery shape: Local implementation, isolated proof, preserved local stack,
  independent before/vision/runtime visual review and fixes.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md)
- [Catalogue contract](../../../contracts/mcp-catalogue-and-profiles.md)
- [Runtime credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Design system](../../desktop-experience/design-system.md)
- [ADR 0003](../../../adr/0003-product-runtime.md)
- [Owner](../../epics/desktop-visual-experience.md)

## Scope

- In scope: All six concepts, shared navigation, global library, inert multi-format
  MCP config import, central owner credential provisioning and explicit inspection.
- Out of scope: Invented image data, arbitrary installation/execution, other users'
  private tasks, replacing operator Vault/runner configuration or granting tool use.
- Blockers: None; user approved the visual and functional concepts. Credential
  provider behavior is established in the runtime amendment before implementation.

## Surface and Interface Changes

- Interfaces: Global definition reads, transient authenticated HTTP inspection,
  exact-revision owner central credential provisioning, `/connectors` route.
- Storage: Existing definition/connection tables; private sibling provider/binding
  files reuse the installation credential writer. No database migration planned.
- Ownership: Existing canonical catalogue, authorization and runtime resolver;
  React presentation uses the existing fonts, brand, theme and route shell.

## Data and Authority

- Inputs: Existing authorized roster, policies, catalogue, runtime and model feeds.
- Authority: Accepted contracts; approved images govern layout only. Secret entry
  is an explicit owner operation and never implies execution rights.
- Blind spots: Generated icon/name/command variations are illustrative. Local file
  storage does not establish hardened shared-service credentials.

## States and Edge Cases

- Loading: Bounded skeletons/feedback preserve page structure.
- Empty: Useful setup and truthful no-data states; no fabricated library cards.
- Error: Clear protected views on failed authority/refresh; preserve safe drafts.
- Blocked: Unsupported config routing or unavailable credential providers explain
  the actual constraint; no secret fallback or automatic execution.
- No-access: Owner library/provisioning, Brain admin writes and per-profile rights
  remain distinct; archive and role loss stop dependent changes.
- Duplicate or replay: Existing approval/create idempotency and connection revision
  fences; stable binding keys for credential retries and rotation.
- Stale data: Policy/connection revisions and current catalogue authorization checked.
- Reconciliation divergence: Historical successful calls are labeled observed;
  missing current sessions never imply server failure.

## Integrations and Runtime Inputs

- Providers: Existing rmcp bounded HTTP metadata inspection and credential adapters.
- Environment: Existing `RECOLLECT_CREDENTIAL_FILE`, optional
  `RECOLLECT_MCP_CREDENTIALS_FILE`; no values recorded.
- Secrets: Private provider files, reference-only bindings, exact response redaction,
  no secret config/database/audit/cache/browser persistence.
- Failure handling: Bounded no-redirect/no-retry metadata calls; provider failures
  fail closed; tool execution and uncertain completion retain existing rules.

## Tests and Acceptance

- Automated: Meaningful parser rejection/multi-format proof, owner/role/archive/
  stale credential denial, no secret reflection, real file-provider resolution,
  focused browser journeys, build/typecheck/design, governance and whitespace.
- Manual: Independent UI agent captures before, compares all six approved images
  to actual 1440 and 1920 local screens, reports gaps and rechecks fixes.
- Acceptance: All six surfaces delivered with real data and usable actions;
  no random advanced/history placement; stable privacy row editing; diagnostics
  secondary; actual explicit metadata success distinguished from configuration.
  Existing data inventory preserved by local rebuild.

## Closeout

- Planned: Six approved concepts and bounded supporting APIs.
- Shipped: All six approved surfaces with real authorized data; global owner
  library; inert bounded JSON/TOML/YAML paste/file imports; explicit authenticated
  metadata inspection; masked central development-provider provisioning; stable
  inline Privacy edits; compact AI permissions and lazy history; observed Agents.
  Locally deployed and independently visually accepted at both requested sizes.
- Not shipped: No agreed acceptance deferred.
- New blockers: None.
- Docs updated: Desktop/catalogue/runtime/knowledge-surface amendments, design
  system, provider runbooks, [dated proof](../../../mappings/desktop-management-concepts-2026-10-05.md),
  epic, archive/active indexes and current handover.
- Validation: Twelve unique focused browser cases pass; credential owner/admin,
  revision/archive, private file, redaction and rotation platform proof plus runtime
  destination safeguard pass. Partial credential failure/retry and config error
  recovery pass without duplicate connection or tool use. Typecheck/design/build,
  182 API operations, Clippy with warnings denied, 32 governance tests, whitespace
  and synchronized CodeGraph pass. Local readiness true; same 16 Brains, two
  connections, one definition, 90 sources and 35 devices. Independent first live
  review plus final independent pixel comparison accepts all six at 1440/1920;
  final capture provenance and geometry are recorded in the dated proof.
- Limits: Secret entry explicitly uses the installation-local development file;
  existing operator Vault/OS/environment aliases remain. No general hardened vault
  enrollment, model-provider call or external tool-success claim. File and database
  effects are separate; a failed credential step can leave a configured connection.
  Container OS-store availability is not presumed. Context7 dependency guidance
  succeeded for Mantine 8 and keyring; primary parser guidance and versions are
  recorded in the mapping. Final diagnostic interaction proof comes from isolated
  tests; subagent recheck used independent inspection of parent live captures.
- Version: N/A; no release policy.
- Commit: Uncommitted; no new commit/push request.
