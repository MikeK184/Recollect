# 0006: Autonomous memory with optional human intervention

Status: accepted

## Decision

Normal Brain operation learns, reconciles, revises and retires memory without
requiring a person to approve individual records. Models interpret permitted
evidence; native workers apply bounded decisions through the canonical mutation
service and enforce retention and erasure. Human review remains an available
override and inspection capability, not a prerequisite for routine operation.

Brain configuration supplies standing authority, content permission and a cost
budget. Automatic decisions record their policy and evidence/model provenance.
They never invent a human reviewer or operational proof. Insufficient evidence
produces an explicit uncertain disposition while unrelated processing continues.

## Why

On 2026-09-14 the user clarified that nobody will manually maintain each memory.
The initial implementation overemphasized proposals and browser acceptance.
Atlas's editing-surface pattern identifies the limit of depending on what a person
can maintain; its trust-state pattern warns about candidates without an effective
promotion/expiry path. These support the decision without mandating every pattern.

## Consequences

The [autonomous maintenance contract](../contracts/memory-autonomous-maintenance.md)
governs this correction. The active procedures/handovers slice includes the closed
maintenance loop before the memory epic can close; the original 29-slice goal
remains intact. Shipped packs retain their historical evidence.

Preserve human overrides, scope, temporal history and deletion fences. Logical
forgetting and physical erasure remain distinct. Models identify unsupported or
superseded memory; retention policy and the native deletion journal control stored
content removal and prevent restoration from undoing it.

## Supersession

Supersedes the foundation/provider-contract assumption that nonliteral model
interpretations and conflicts inherently require human acceptance. Supplements
[ADR 0005](0005-canonical-claims-and-time.md), retaining canonical storage, history
and evidence boundaries. Legacy explicit-learning mode remains usable.
