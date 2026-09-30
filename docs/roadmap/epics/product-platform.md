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
[archived pack](../execution/archive/platform-bootstrap.md). Durable work is also
delivered in its [archived pack](../execution/archive/platform-durable-work.md).
Team access is delivered in its [archived pack](../execution/archive/platform-team-access.md).
Device pairing is delivered in its [archived pack](../execution/archive/platform-device-pairing.md).

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

## Decisions Before Implementation

- Record the selected packaging/storage architecture in an ADR and define the
  first slice's interfaces, minimal data model, migrations and compatible dependency ranges.
- Specify owner bootstrap/recovery, sessions, Brain ownership/grants, audit
  boundaries and non-owner database access before their handlers are implemented.
- Specify invitation/OIDC mapping, membership refresh and device pairing/revocation
  before the corresponding slices. These refine the accepted login direction.

## Slice Map

Small-fix exception, 2026-09-14: claims regression exposed a valid numeric pairing
code being coerced by router JSON search parsing and removed by route validation.
The existing pairing contract defines an opaque eight-hex-character identifier.
Preserve its exact string through parsing/stringifying; deterministic browser
fixtures cover numeric, leading-zero and exponent-shaped codes alongside the
real native pairing proof. No schema, wire protocol or new product decision changes.

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `platform-bootstrap` | shipped | adr-backed, contract-backed | pack | Runnable Rust service and React UI with selected databases, local owner login, Brain administration and two-Brain isolation through an auditable command boundary |
| `platform-durable-work` | shipped | contract-backed | pack | Atomic mutation/audit/outbox support, idempotent bounded workers and explicit job/projection state |
| `platform-team-access` | shipped | contract-backed | pack | Invited local accounts, optional OIDC, effective grants and tested internal/external revocation behavior |
| `platform-device-pairing` | shipped | contract-backed | pack | Native companion enrollment with individually revocable credentials and shared protocol types |
| `desktop-experience-contracts` | shipped | adr-backed, contract-backed | pack | Accepted desktop/Ask decisions, domain-owned scope and decision-complete implementation packs |
| `platform-desktop-shell` | shipped | adr-backed, contract-backed | pack | Shared light tokens, self-hosted fonts, SVG logo, global views and contextual Brain routes |
| `platform-brain-deletion` | planned | adr-backed, contract-backed | pack | Irreversible Brain-wide erasure through the canonical mutation, journal and cleanup path |
| `desktop-knowledge-surface` | planned | adr-backed, contract-backed | pack | Tiered sidebar, merged Memory/Sources/Graph/Repositories surface, shared lineage inspector, graph chrome and list density |

## Slice Dependencies

Only named predecessor slices are required; completion of an entire upstream
epic is not an implicit prerequisite. Dependency rows across the roadmap form
one acyclic implementation order.

| Slice ID | Predecessors |
| --- | --- |
| `platform-bootstrap` | None |
| `platform-durable-work` | `platform-bootstrap` |
| `platform-team-access` | `platform-bootstrap` |
| `platform-device-pairing` | `platform-bootstrap` |
| `desktop-experience-contracts` | `operations-integrated-evaluations` |
| `platform-desktop-shell` | `desktop-experience-contracts` |
| `platform-brain-deletion` | `platform-bootstrap`, `memory-retention-and-erasure` |
| `desktop-knowledge-surface` | `platform-desktop-shell`, `evidence-desktop-workflows`, `memory-desktop-workflows`, `graph-desktop-workspace` |

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

## Desktop implementation evidence, 2026-09-26

The [dated implementation mapping](../../mappings/desktop-experience-implementation-2026-09-26.md)
records implemented routes/assets and the final local image separately from
remaining cross-domain acceptance. The [platform-desktop-shell closeout](../execution/archive/platform-desktop-shell.md)
records this owner’s completed checks. The [desktop guide](../../runbooks/desktop-experience.md)
documents current function locations. Other owners keep unfinished criteria active;
prior shipped domain records remain historical evidence rather than redesign proof.

## 2026-09-29 deletion and surface amendment

The user decided that a Brain must be permanently deletable with total erasure,
that Memory, Sources and Graph must merge into one improved surface, and that
Agents and Connections must separate. [ADR 0016](../../adr/0016-brain-deletion.md)
and [ADR 0017](../../adr/0017-desktop-knowledge-and-ask-experience.md) record those
decisions; [brain deletion](../../contracts/platform-brain-deletion.md) and the
[tiered surface](../../contracts/desktop-knowledge-surface.md) contracts specify them.

This epic reopens from complete to active because it owns the deletion command and
the coordinated presentation layer spanning the memory, evidence and graph domains.
Domain authority does not move here: each capability keeps its handlers,
permissions, retention and proof obligations, following the coordination precedent
`desktop-experience-contracts` already established.
