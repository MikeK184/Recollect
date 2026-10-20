# Execution Packs

The epic slice row is the default record: Execution `inline: <acceptance;
validation>`. Add a pack only for risky work: a schema or migration change with
restore or compatibility impact; a new external integration or a paid or frozen
evaluation campaign (cost approval, frozen input hashes); a security,
credential or authz change no accepted contract fully specifies; or
multi-session work with open decisions.

Create `active/<slice-id>.md` from [template.md](template.md) after the slice
exists, with Status `planned`, `in-progress` or `blocked`, and keep the
[active index](active/README.md) to packs and statuses. Execution values:
`pack` (active pack exists), `inline: ...` (no pack), `archived` (shipped; pack
deleted at closeout). Closeout follows [the lifecycle](../../README.md). Shipped
packs before 2026-10-19 are in
[Git history](https://github.com/MikeK184/Recollect/tree/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive).
