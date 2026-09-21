# Execution Packs

Packs are decision-complete specifications under an owning epic. Use
[template.md](template.md) at `active/<slice-id>.md` after the slice exists in
the epic's slice map. Keep the exact filename throughout its lifecycle.

## Metadata and content

Every pack declares `Status`, `Owning epic` (a repo-relative path), and
`Work type` (`governance` or `product`). Required sections are Summary,
Governing Sources, Scope, Surface and Interface Changes, Data and Authority,
States and Edge Cases, Integrations and Runtime Inputs, Tests and Acceptance,
and Closeout. Retain all template fields; use `N/A: <reason>` when irrelevant.
Fill them before implementation, except actual closeout evidence which is
completed after validation. Do not keep unresolved placeholders in an
in-progress pack's implementation specification.

Governance means repo docs and developer tooling. Product means application
or runtime behavior. Pending foundations cannot govern either kind, and
product implementation waits until the foundation baseline is accepted.

## State lifecycle

| Status | Location | Meaning |
| --- | --- | --- |
| planned | active | Defined slice; specification may still be refined |
| in-progress | active | Required decisions resolved; implementation underway |
| blocked | active | Named unresolved blocker and next action |
| shipped | archive | Agreed slice delivered with validation and closeout evidence |

Use planned → in-progress → shipped, with blocked/resumed work reflected in
both pack and epic. Resume blocked work as planned or in-progress only after
its dependencies permit it. A shipped pack is delivery history; later changes
normally belong to a new slice. Do not delete a blocked pack to hide unfinished
work. If scope is cancelled, record the user's decision and reconcile the
roadmap explicitly; cancellation is not shipment.

## Indexes and closeout

[Active](active/README.md) and [archive](archive/README.md) indexes list each
pack once with its actual status. Reconcile the pack, epic slice, both indexes,
and epic overview in the same change. Follow [the closeout rules](../../README.md).

Shipped closeout includes Planned, Shipped, Not shipped, New blockers, Docs
updated, Validation, Version, and Commit. Use `N/A: no release policy` and
`uncommitted` when accurate. Document partial deferrals without marking unmet
acceptance criteria as shipped.
