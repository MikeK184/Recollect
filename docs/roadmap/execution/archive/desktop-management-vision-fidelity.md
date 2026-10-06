# Management vision fidelity

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Match all six approved images' layout, typography, icons, formatting and
  inline edit controls, correcting the rejected implementation.
- Non-goals: New backend semantics, fictional inventory, external publication.
- Delivery shape: UI-agent implementation, independent screenshot review, local
  runtime rebuild and updated before/vision/after artifact.

## Governing Sources

- [Desktop contract](../../../contracts/desktop-experience.md)
- [Design system](../../desktop-experience/design-system.md)
- [Catalogue contract](../../../contracts/mcp-catalogue-and-profiles.md)
- [Runtime credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Retention](../../../contracts/memory-retention-and-erasure.md)
- [Vision](../../../foundation/vision.md)
- [Owning epic](../../epics/desktop-visual-experience.md)

The user's October 5 correction explicitly prioritizes the approved visual
reference. Final images in `output/imagegen/2026-10-05-management-concepts-01a10b36`
are `01-connections.png`, `02-add-connection.png`, `03-global-connectors-v2.png`,
`04-privacy.png`, `05-ai-permissions-v2.png` and `06-agents.png`. Earlier variants
are not the target. Generated records, marks and commands remain illustrative.

## Scope

- In scope: Six screens, associated edit states, shared shell geometry, typography,
  field layout, icon sizing, and comparison artifact using existing UI primitives.
- Out of scope: Changing credentials, grants, captured data or model permissions;
  fabricated connector/agent rows; research checkout changes; new dependencies.
- Blockers: None. Existing contracts resolve behavior; user resolves visual direction.

## Surface and Interface Changes

- Interfaces: Existing routes and APIs; no endpoint changes. Preserve form parsing,
  inspection, review, explicit saves and lazy diagnostics.
- Storage: No migration or policy reset. Only transient tab navigation may retain
  the last actually selected Brain identifier, validated against current grants
  and cleared on session/account change; no protected payload persistence.
- Ownership: UI worker owns management components/styles; root owns documentation,
  runtime integration and comparison; independent agent owns visual findings.

## Data and Authority

- Inputs: Actual authorized catalogue, connections, roster, policies and model coverage.
- Authority: Existing contracts and explicit user vision correction. Actual labels
  and counts override fictional image content without changing the composition.
- Blind spots: Generated font rasterization varies; compare at the reference's
  1586 × 992 dimensions and practical 1440/1920 desktop widths.

## States and Edge Cases

- Loading: Preserve bounded skeletons and structure.
- Empty: Compact true empty/filter states without invented cards.
- Error: Keep parse errors and failed-refresh authority gates.
- Blocked: Actual provider/execution constraints stay visible in context.
- No-access: Existing owner/Brain admin/per-agent scope and disabled actions remain.
  Global Connectors can retain actual accessible Brain navigation while catalogue
  reads/writes stay installation scoped. Never select a first Brain implicitly.
- Duplicate or replay: Existing idempotency and credential retries remain.
- Stale data: Preserve base revisions, invalidation and cleared protected views.
- Reconciliation divergence: Configuration, metadata inspection and observed use
  retain separate labels; styling must not imply health or tool permission.

## Integrations and Runtime Inputs

- Providers: Existing APIs and credential adapters; no external visual test calls.
- Environment: Existing local Compose configuration; no new variables.
- Secrets: Preserve masking, transient drafts and reference-only configuration;
  never record secrets in screenshots or artifacts.
- Failure handling: Existing bounded explicit inspection and fail-closed authorization.

## Tests and Acceptance

- Automated: Focused existing management/config tests, web typecheck/build/design,
  `./scripts/validate.sh`, whitespace and CodeGraph synchronization.
- Manual: Compare all six final reference images to actual rendered screens, including
  retention edit, pasted-config authentication and connection edit. Independent agent
  reports concrete gaps, worker fixes them, reviewer reinspects fresh captures.
- Acceptance: Header-aligned connection inspector; broad compact config dialog with
  format tabs, numbered editor, horizontal credentials and review footer; compact
  catalogue plus prominent import rail; retention-first stable compact Privacy;
  provider/memory strip and separate coverage/models/options AI cards; weighted
  roster and concise setup/memory cards. No lost actions or clipped fields at desktop
  sizes. HTML shows original, final vision and fresh actual image bytes; historical
  rejected captures remain available.

## Closeout

- Planned: Six corrections, independent review and refreshed comparison.
- Shipped: All six management compositions corrected by the UI agent, rebuilt on
  the existing local stack and accepted by the independent reviewer at 1586 × 992,
  1440 × 900 and 1920 × 1080. Header/row actions, numbered config/credential fields,
  global import rail, stable compact Privacy, AI policy/model cards and weighted
  Agent roster/setup utilities are delivered. The refreshed self-contained
  comparison embeds 88 unchanged images and retains rejected states.
- Not shipped: Fictional image records, generated commands or credential-provider
  claims; external tool/model/server connectivity; phone/tablet refinement.
- New blockers: None.
- Docs updated: Desktop contract, design system, epic, active/archive indexes,
  prior concept closeout correction, dated mapping, comparison README and handover.
- Validation: Twelve focused browser cases passed in one sequential isolated run;
  production build, typecheck/design checks, independent final pixels and lower
  controls, populated diagnostics, comparison image fingerprints and browser
  interactions passed. Local readiness true; exact existing Brain/connection/
  definition/device identities preserved. Final governance and whitespace checks
  passed (32 governance cases); results are recorded in the dated mapping,
  alongside scope and evidence limits. Whitespace checks also passed.
- Version: N/A: no release policy.
- Commit: Uncommitted.
