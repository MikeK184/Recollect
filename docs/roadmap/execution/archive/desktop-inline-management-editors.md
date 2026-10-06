# Inline management editors

Status: shipped
Owning epic: `docs/roadmap/epics/desktop-visual-experience.md`
Work type: product

## Summary

- Goal: Stable connection editing and Connections-style tool-group cards.
- Non-goals: Implicit tool execution or coupling independent powers.
- Delivery shape: Local UI over existing canonical API commands.

## Governing Sources

- [Desktop](../../../contracts/desktop-experience.md)
- [Catalogue](../../../contracts/mcp-catalogue-and-profiles.md)
- [Credentials](../../../contracts/mcp-runtime-and-credentials.md)
- [Provider policy](../../../contracts/memory-provider-policy-and-learning.md)
- [Owner](../../epics/desktop-visual-experience.md)

## Scope

- In scope: Connection inspector fields, stable sections/tabs/footer, card config and
  people grants, independent inline AI permission edits owned by model worker.
- Out of scope: Automatic tests, external tools/provider calls, unrelated dirty edits.
- Blockers: None; user-approved cards and icon permissions govern composition.

## Surface and Interface Changes

- Interfaces: Existing revisioned connection/profile PUT and independent grant commands.
- Storage: Existing persistence; no new combined transaction claim.
- Ownership: Connection config requires Brain administration; group config Manage;
  grants Share; AI policy uses existing current Brain admin authority.

## Data and Authority

- Inputs: Authorized summaries/detail, enrolled accounts/groups, current effective rights.
- Authority: Canonical API rechecks all rights; Use never inherited from admin/Manage/Share.
- Blind spots: Group membership expires; direct grant removal may retain inherited rights.

## States and Edge Cases

- Loading: Card skeleton/detail feedback keeps identity and structure.
- Empty: Truthful empty groups/connections/people; real Add controls.
- Error: Retain safe drafts and show configuration/grant partial save outcomes.
- Blocked: Incompatible environments and unavailable credentials prevent invalid save.
- No-access: Disable only unavailable powers; hide sensitive detail on failed refresh.
- Duplicate or replay: Config then sequential grants; track completed writes and retry only unfinished.
- Stale data: Revision conflict blocks config save, retains draft, explicit Reload required.
- Reconciliation divergence: Effective inherited rights separate from editable direct/group grants.

## Integrations and Runtime Inputs

- Providers: Existing canonical API only; Save never tests a connection/model.
- Environment: N/A; no additional runtime input.
- Secrets: Retain masked credential entry and explicit provisioning retry without config replay.
- Failure handling: Truthful partial saves, current authority polling and existing revision fences.

## Tests and Acceptance

- Automated: Manage/Share independence, green/red labelled icons, cancellation,
  revision conflict, partial grants retry, environment restrictions; focused browser/build
  checks and `./scripts/validate.sh`.
- Manual: Independent Connections/card comparison at laptop and large desktop widths.
- Acceptance: Inline values, MCPs and people fit inside approved card; permission icons
  beside name; no duplicated group title/form or disappearing inspector sections.

## Closeout

- Planned: Stable connection inspector/cards and independent inline edits.
- Shipped: Connections-style sage cards with MCPs/People, adjacent green/red
  effective icons and explicitly labelled Direct controls; stable connection
  Save/Cancel, retained conflicts, authority-specific drafts and guarded selection;
  partial-save receipts survive repeated retry. Inline AI controls preserve the
  approved layout and save through canonical revisioned policy commands.
- Not shipped: N/A within the accepted editor scope; Save never tests an MCP
  or makes a paid model call.
- New blockers: None.
- Docs updated: Desktop/catalogue/runtime amendments, pack, epics/index, evidence and handoff.
- Validation: Isolated MCP independent-power/RLS and schema/reconfiguration
  fixtures pass. CUA checks Save/Cancel, selection guards, 390px stable inspector,
  retained revision conflict/Reload/Save, second MCP persistence, repeated partial
  grant retry receipts and synthetic AI checkbox persistence after reload.
  Independent actual screenshots at laptop/large desktop scale pass. Frontend
  build/type/design and Clippy pass; browser regression sources remain unexecuted.
- Evidence: [Coordinated delivery and limits](../../../mappings/desktop-browser-management-2026-10-05.md).
- Version: N/A; no release requested.
- Commit: Uncommitted.
