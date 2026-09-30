# Active Execution Packs

Statuses here are planned, in-progress, or blocked. Move shipped packs to the
archive and reconcile both indexes with the owning epic.

The user approved the complete desktop experience on 2026-09-26 and requested
implementation. The authority-only slice is [shipped](../archive/desktop-experience-contracts.md);
the [desktop shell](../archive/platform-desktop-shell.md) and
[MCP setup](../archive/mcp-desktop-setup.md) are also shipped. Six product/proof
slices remain in progress with
[deployed implementation and remaining verification](../../../mappings/desktop-experience-implementation-2026-09-26.md). The original 29 slices, Atlas capture repair and root Compose work
remain delivered history in the [archive](../archive/README.md). See the
[epic index](../../epics/index.md) for ownership and dependency order.

| Pack | Status |
| --- | --- |
| [opencode-project-setup](opencode-project-setup.md) | in-progress |
| [mcp-codex-plugin](mcp-codex-plugin.md) | in-progress |
| [evidence-desktop-workflows](evidence-desktop-workflows.md) | in-progress |
| [memory-desktop-workflows](memory-desktop-workflows.md) | in-progress |
| [graph-desktop-workspace](graph-desktop-workspace.md) | in-progress |
| [retrieval-ask-experience](retrieval-ask-experience.md) | in-progress |
| [operations-desktop-activity](operations-desktop-activity.md) | in-progress |
| [desktop-experience-acceptance](desktop-experience-acceptance.md) | in-progress |
| [operations-longmemeval-bench](operations-longmemeval-bench.md) | blocked |
| [platform-brain-deletion](platform-brain-deletion.md) | planned |
| [desktop-knowledge-surface](desktop-knowledge-surface.md) | planned |
| [desktop-ask-primary](desktop-ask-primary.md) | planned |
| [desktop-connection-authority](desktop-connection-authority.md) | planned |
| [desktop-assurance-pulse](desktop-assurance-pulse.md) | planned |

The 2026-09-28 [managed experience correction](../archive/memory-managed-experience.md)
is locally delivered; it does not close the six acceptance packs above.

The [direct MCP setup](../archive/mcp-direct-connections.md),
[public retrieval benchmark](../archive/operations-public-benchmark.md) and
[Atlas lifecycle proof](../archive/operations-atlas-lifecycle-proof.md) are also
locally delivered with dated proof; they do not close the six acceptance packs.
The [LongMemEval answer-level run](operations-longmemeval-bench.md) is blocked
on explicit user cost approval under the
[proposed protocol](../../../contracts/operations-longmemeval-protocol.md).

The [SWEG Codex credential repair](../archive/mcp-direct-auth-repair.md) is also
delivered with installed-host proof and deployed Keychain/environment guidance.

The five packs added on 2026-09-29 implement the user's deletion, merged-knowledge,
Ask-default and wiring-split decisions under
[ADR 0016](../../../adr/0016-brain-deletion.md) and
[ADR 0017](../../../adr/0017-desktop-knowledge-and-ask-experience.md). They are
planned specifications, not delivery claims: implementation follows the
[desktop plan](../../desktop-experience/README.md) dependency order, with
`desktop-ask-primary` behind `retrieval-ask-experience` and all five behind
`desktop-experience-acceptance`.
