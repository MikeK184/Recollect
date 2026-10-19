# Product Platform and Access

Status: active

## Purpose

Provide the runnable Rust/React foundation and shared identity, Brain access,
device pairing and durable-work primitives used by every product capability.
Personal use and internal-team use share one application and permission model.

## Governing Sources

- [Desktop ADR](../../adr/0014-desktop-experience-and-answers.md)
- [Desktop contract](../../contracts/desktop-experience.md)
- [Knowledge surface ADR](../../adr/0017-desktop-knowledge-and-ask-experience.md)
- [Tiered surface contract](../../contracts/desktop-knowledge-surface.md)
- [Brain deletion ADR](../../adr/0016-brain-deletion.md)
- [Brain deletion contract](../../contracts/platform-brain-deletion.md)
- [Runtime ADR](../../adr/0003-product-runtime.md)
- [Local password hashing ADR](../../adr/0024-local-password-hashing.md)
- [Bootstrap contract](../../contracts/platform-bootstrap.md)
- [Durable-work contract](../../contracts/platform-durable-work.md)
- [Team access contract](../../contracts/platform-team-access.md)
- [Device contract](../../contracts/platform-device-pairing.md)

- [Vision: deployment and access](../../foundation/vision.md#deployment-and-access-model)
- [Selected stack and packaging](../../foundation/techstack.md#selected-components-and-packaging)
- [Engineering principles: identity and authority](../../foundation/engineering-principles.md#bind-identity-scope-and-authority-explicitly)

## Dependencies and Boundaries

This is the first product epic. Repository governance and developer tooling are
already available. Platform-bootstrap is delivered locally under its accepted ADR/contract and
[archived pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/platform-bootstrap.md). Durable work is also
delivered in its [archived pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/platform-durable-work.md).
Team access is delivered in its [archived pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/platform-team-access.md).
Device pairing is delivered in its [archived pack](https://github.com/MikeK184/Recollect/blob/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive/platform-device-pairing.md).

Own the Cargo/server/companion structure, same-origin React/Vite shell, local
Compose topology, database clients/migrations, local owner bootstrap, Brain
administration and shared authorization primitives. Resolve a compatible Rust/UI/
PostgreSQL/pgvector/Neo4j/GDS set under ADR 0003 and prove actual calls between components.
Basic command authorization and mutation audit accompany the first Brain writes.

The durable-work slice supplies transactional outbox, idempotency and bounded
worker primitives; downstream epics own their domain mutations and consumers.
Keep the companion independent of server database drivers. UI shell ownership
does not centralize every feature screen here: each capability owns its UI,
errors, authorization and proof.

Operations owns hardened shared deployment and recovery drills. Workspace scope,
memory semantics and MCP connection/profile resources remain with their epics.
No mandatory OIDC provider, customer federation, new secret store or replacement
database engine is introduced.

## Slice Map

Inline slice (no pack), 2026-09-14: claims regression exposed a valid numeric pairing
code being coerced by router JSON search parsing and removed by route validation.
The existing pairing contract defines an opaque eight-hex-character identifier.
Preserve its exact string through parsing/stringifying; deterministic browser
fixtures cover numeric, leading-zero and exponent-shaped codes alongside the
real native pairing proof. No schema, wire protocol or new product decision changes.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `platform-bootstrap` | shipped | adr-backed, contract-backed | archived | Runnable Rust service and React UI with selected databases, local owner login, Brain administration and two-Brain isolation through an auditable command boundary |
| `platform-durable-work` | shipped | contract-backed | archived | Atomic mutation/audit/outbox support, idempotent bounded workers and explicit job/projection state |
| `platform-team-access` | shipped | contract-backed | archived | Invited local accounts, optional OIDC, effective grants and tested internal/external revocation behavior |
| `platform-device-pairing` | shipped | contract-backed | archived | Native companion enrollment with individually revocable credentials and shared protocol types |
| `desktop-experience-contracts` | shipped | adr-backed, contract-backed | archived | Accepted desktop/Ask decisions, domain-owned scope and decision-complete implementation packs |
| `platform-desktop-shell` | shipped | adr-backed, contract-backed | archived | Shared light tokens, self-hosted fonts, SVG logo, global views and contextual Brain routes |
| `platform-brain-deletion` | shipped | adr-backed, contract-backed | archived | Irreversible Brain-wide erasure through the canonical mutation, journal and cleanup path |
| `desktop-knowledge-surface` | shipped | adr-backed, contract-backed | archived | Tiered sidebar, merged Memory/Sources/Graph/Repositories surface, shared lineage inspector, graph chrome and list density |
| `platform-local-credential-hardening` | in-progress | adr-backed, contract-backed | pack | Argon2id hashing of invited local account passwords, automatic conversion of legacy plaintext entries and a fail-closed strict credential-file read |

## Completion Criteria

- The selected stack builds from resolved compatible dependencies under ADR 0003 and starts through a
  documented local command; UI/API and both databases answer real checks.
- Owner bootstrap, login, invitations/OIDC and device pairing work through actual
  handlers. Unauthorized direct requests fail and authorized equivalents succeed.
- Two Brains remain isolated; removing one grant does not falsely imply other
  ownership or inherited grants were removed. Profile use remains separately granted.
- Crash/retry tests preserve canonical mutation/audit/outbox atomicity and visible
  job failure. No test result is inferred from middleware or a schema alone.
- Publish focused validation and a runnable development procedure. This epic
  completing does not establish memory, graph-analytics or MCP product readiness.

## Current status

Accounts, Brain permissions, auditable durable commands, device identity and canonical Brain deletion are delivered. The original platform slices have no unfinished acceptance in this epic. New automatic mapping must extend existing isolation/deletion behavior under the graph organization pack.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/a0aea9dfa26a6f9f348423b32b3a0adbaac1196d/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/docs/roadmap/epics/product-platform.md).
Dated test/deployment observations are not a current runtime health claim.
