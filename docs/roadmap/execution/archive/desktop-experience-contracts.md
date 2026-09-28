# Accepted desktop and Ask authority

Status: shipped
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: governance

## Summary

- Goal: Make the user's approved desktop redesign decision-complete for its existing domain owners before significant code changes.
- Non-goals: Application implementation, shipment claims from design images, new framework or extra approval ceremonies.
- Delivery shape: Accepted ADR/contracts, approved plan and nine domain-owned execution packs with consistent indexes.

## Governing Sources

- [Governance](../../../contracts/repository-governance.md) and [documentation lifecycle](../../../README.md)
- [Accepted vision](../../../foundation/vision.md), [stack](../../../foundation/techstack.md) and [principles](../../../foundation/engineering-principles.md)
- [ADR 0014](../../../adr/0014-desktop-experience-and-answers.md)
- [Desktop contract](../../../contracts/desktop-experience.md) and [answer contract](../../../contracts/retrieval-answers.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: Record explicit 2026-09-26 user approval, routes/tokens/logo, feature placement, list-query semantics, Ask authority/lifecycle/retention, dependency ownership and meaningful tests.
- Out of scope: Rewriting shipped history, changing unrelated root Compose work, reference checkout writes and product code.
- Blockers: None; the user approved the complete proposal and implementation. Routine typed-interface details are resolved in contracts and packs.

## Surface and Interface Changes

- Interfaces: Documentation specifies twelve routes, source/claim q filters and answer lifecycle; domain implementation follows its own pack.
- Storage: N/A: this governance slice writes Markdown only; answer metadata storage is specified for its retrieval owner.
- Ownership: Platform coordinates authority; evidence, memory, graph, MCP, retrieval and operations retain behavior and proof ownership.

## Data and Authority

- Inputs: User-approved proposal, source-reviewed current capability inventory and accepted domain contracts.
- Authority: User decision and accepted ADR/contracts; screenshots and mappings remain illustrative/evidence-only.
- Blind spots: Documentation validation proves structure, not product behavior, provider access or deployed connectivity.

## States and Edge Cases

- Loading: N/A: no interactive application state is delivered by documentation authoring.
- Empty: N/A: complete sources exist; missing authority would be an explicit blocker rather than an empty specification.
- Error: Broken links or lifecycle inconsistencies are repaired without weakening validation.
- Blocked: A newly discovered unresolved behavior is recorded on the dependent slice while independent authorized work continues.
- No-access: N/A: no new runtime access path; domain access boundaries are explicitly preserved in contracts.
- Duplicate or replay: Unique slice IDs and one canonical accepted contract prevent competing active specifications.
- Stale data: Historical implementation evidence retains its date; approval never relabels planned behavior shipped.
- Reconciliation divergence: Pack/epic/index and accepted amendments must agree before closeout.

## Integrations and Runtime Inputs

- Providers: N/A: documentation authoring performs no model/runtime integration.
- Environment: N/A: no runtime variables are changed by this slice.
- Secrets: No credentials or provider payloads enter documentation.
- Failure handling: Record failed dependency verification and limit claims; authority gaps block only dependent implementation.

## Tests and Acceptance

- Automated: Governance lint, all checker tests through ./scripts/validate.sh and git diff --check; unique ownership/dependency/pack link review.
- Manual: Compare all accepted decisions, page responsibilities and Ask failure matrix with domain packs; verify no unresolved placeholders.
- Acceptance: Nine decision-complete packs and owning slice maps exist; independent workers can implement without inventing product authority; application shipment stays unclaimed.

## Closeout

- Planned: Complete approved-desktop authority and execution route.
- Shipped: Accepted ADR/contracts, domain amendments and all nine decision-complete packs are present; the authority-only slice is complete. Product implementation and runtime acceptance remain in their separate packs.
- Not shipped: Product code/runtime acceptance belongs to the eight implementation/proof slices.
- New blockers: None.
- Docs updated: ADR 0014, desktop/answer/domain amendments, approved plan, seven epic records and execution indexes; [authority handoff](../../../mappings/desktop-experience-authority-2026-09-26.md) and [implementation boundary](../../../mappings/desktop-experience-implementation-2026-09-26.md).
- Validation: 2026-09-26 ./scripts/validate.sh passed governance lint and all 32 tests; git diff --check -- docs passed. An initial non-authoritative foundation-index link was corrected to the three accepted foundation documents before the passing run.
- Version: N/A: documentation-only delivery, no release.
- Commit: uncommitted.
