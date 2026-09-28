# Desktop implementation authority handoff

Observed: 2026-09-26
Confidence: verified

## Sources and Method

The user explicitly accepted the complete desktop design package and requested
implementation of all views, consistent fonts/icons/SVGs and an original SVG
Recollect logo inspired by remembering or gathering again. This follows the
[review mapping](desktop-experience-review-2026-09-26.md); approval is a new
instruction, not an inference from generated images.

Read the repository guide, documentation router/maintainer skills, foundations,
existing domain contracts/epics and the full approved plan. Coordinated exact
Ask/list wire details with the implementing backend owner before authoring its
pack. Wrote documentation only, preserving the prior root Compose work and all
reference checkouts. Application implementation by other workers is concurrent
and is not validated by this handoff.

## Observations

- [ADR 0014](../adr/0014-desktop-experience-and-answers.md) is accepted.
- [Desktop](../contracts/desktop-experience.md) and
  [temporary answer](../contracts/retrieval-answers.md) contracts are accepted.
- The approved plan/design documents now record the explicit acceptance date.
- Nine unique slices have decision-complete active packs and existing domain
  owners. Seven product epics are active again for this bounded expansion;
  earlier shipped records are retained unchanged.
- Existing model, investigation, graph, source/claim search and retention
  contracts link the new consumer/presentation boundaries without changing their
  original domain authority.

Final agreed Ask wire uses a stable request UUID, question up to 512 UTF-8 bytes
and RecallRequest. It preserves an explicit advanced query or derives bounded
literal terms without a model rewriter. Exact canonical fragments are rechecked
without reranking/re-embedding. Metadata GET/cancel remains account-owned; no
transcript/prompt/output is persisted. A minimal replay tombstone prevents an
additional answering attempt and, when selected, additional semantic-query
embedding under the same request ID. Question/answer display is at most four
temporary browser-memory turns. These are selected contracts, not observed
runtime behavior in this document.

Source/Memory list `q` is trimmed to at most 200 UTF-8 bytes, case-insensitive
literal containment before pagination over current permitted source title or
claim subject/predicate/value/rationale. Raw source-body search is not implied.

## Translation and Limits

Implementation can proceed from the owning contracts/packs without further
permission ceremonies. Shared shell and independent backend work need not wait
for unrelated visual integration. Final acceptance still requires every selected
journey, meaningful exclusion with positive controls, real desktop proof and
preserved runtime inventory. No placeholder panel, mock data or configured
provider is delivery evidence.

No pending foundation decision is introduced. Saved/action-capable Ask, arbitrary
provider/connector installation, mobile and dark mode remain explicit deferrals.
No commit, push, release or external deployment is implied by this handoff.

## Validation

- `./scripts/validate.sh`: governance lint and all 32 checker tests passed.
- Initial lint failure: a governing link targeted `foundation/README.md`, an
  index without accepted status. Replaced it with the three actual accepted
  foundation documents; validation then passed without modifying the checker.
- `git diff --check -- docs`: passed.
- Reviewed all nine slice IDs/owners, active index and epic overview, exact API
  bounds and deferral reconciliation. No product runtime/provider result is
  claimed by this documentation check.

## Follow-up

Root implementation coordinates actual feature/API/browser evidence and final
pack closeout. The [active index](../roadmap/execution/active/README.md) is the
current work inventory. Preserve this authority check as dated evidence rather
than replacing it with a later shipment assertion. Version N/A; uncommitted.
