# <Epic title>

Status: planned

## Purpose

<Capability boundary and intended outcome.>

## Governing Sources

<Links to accepted ADRs/contracts and relevant accepted foundation documents.>

## Dependencies and Boundaries

<Sequencing, unresolved decisions, exclusions, and downstream handoffs.>

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `<unique-slice-id>` | planned | needs-contract | pack | <Concrete slice outcome> |

Slice statuses: planned, in-progress, blocked, shipped. IDs use lowercase words
separated by hyphens and are unique across the repo. Execution is `pack`, or
`small-fix: <reason>` for a qualifying isolated fix that needs no pack.
Planned/blocked slices may await a pack; implementation/shipping requires one
unless the documented small-fix exception applies. Evidence labels are
comma-separated values from the index legend. Epic statuses are planned,
active, blocked, or complete; complete requires every slice shipped.
