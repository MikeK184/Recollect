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
| `<unique-slice-id>` | planned | needs-contract | inline: <acceptance; validation> | <Concrete slice outcome> |

Slice statuses: planned, in-progress, blocked, shipped. IDs use lowercase words
separated by hyphens and are unique across the repo. Execution is one of:
`pack` (an active pack `execution/active/<slice-id>.md` exists; only for
planned, in-progress or blocked slices), `inline: <acceptance; validation>` (no
pack; the row carries acceptance and validation) or `archived` (shipped; the
pack was removed at closeout and stays in Git history). Packs are required only
for the risky work listed in the [execution guide](../execution/README.md).
Evidence labels are comma-separated values from the index legend. Epic statuses
are planned, active, blocked, or complete; complete requires every slice shipped.
