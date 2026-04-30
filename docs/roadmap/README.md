# Roadmap

[Epics](epics/index.md) own capability boundaries and sequencing.
[Execution packs](execution/README.md) make individual slices concrete enough
to implement. Neither may override accepted ADRs or contracts.

The accepted foundations have seven product epics, covering platform,
evidence/workspaces, memory/review, retrieval, graphs, MCP/Vault and operational
readiness. The epic index records the first slice, staged handoffs and ownership
of the seven required memory capabilities. Per-epic Slice Dependencies tables
specify prerequisites without imposing whole-epic completion barriers.

Fourteen of the original 29 product slices are delivered locally, including exact
and lexical recall. An additional delivered Atlas audit repair slice covers the
capture/learning integration. Semantic retrieval is next. The epic and execution indexes track current
implementation, open acceptance and local validation evidence. External deployment
and commits are outside the active goal.

Create the epic slice before its pack. Keep blockers visible and resolve
`needs-adr` or `needs-contract` before implementation. Update the epic and
execution indexes when status or ownership changes.
