# One typed Brain admission

Status: in-progress
Owning epic: `docs/roadmap/epics/product-platform.md`
Work type: product
Implementation: authorized 2026-10-19 by the repository owner (design final, both parts)

## Summary

- Goal: Every request or worker transaction that touches one Brain is admitted
  by one typed, documented function in one database round trip after
  authentication, with one canonical order (transport, principal, visibility,
  role, archive fence, surface locks), instead of five per-surface helpers and
  about 110 inline combinations of `require_role`, `require_writer` and archive
  reads. Part 2 then gives every surface the same status and error code.
- Non-goals: Change who is allowed (roles, ownership, device and browser-only
  rules), RLS policies, isolation levels, the Brain lock namespace
  (`pg_advisory_xact_lock(73241029,admission_key)` plus the Brain row lock),
  profile or publication rights, or any transaction that has no Brain target
  (the 23 `auth.tx` calls without a Brain check stay).
- Delivery shape: Part 1 (W0, W1) is behaviour-preserving and ships a golden
  status matrix first. Part 2 (W2, W3) migrates the remaining sites and flips
  the matrix rows that the owner decided to unify.

## Governing Sources

- [Team access contract](../../../contracts/platform-team-access.md)
- [Device pairing contract](../../../contracts/platform-device-pairing.md)
- [Brain deletion contract](../../../contracts/platform-brain-deletion.md)
- [MCP catalogue and profiles contract](../../../contracts/mcp-catalogue-and-profiles.md)
- [MCP managed skills contract](../../../contracts/mcp-managed-skills.md)
- [MCP agent coordination contract](../../../contracts/mcp-agent-coordination.md)
- [Evidence collections contract](../../../contracts/evidence-collections.md)
- [Owning epic](../../epics/product-platform.md)

## Scope

- In scope: A `db::admission` module and migration `070_brain_admission.sql`;
  a pure `decide()` function with unit tests for every branch; a golden
  matrix platform test recorded against the unmodified code; re-implementation
  of the five helpers and the `evidence.rs` clone on it with zero behaviour
  change (W1, part 1); then the inline `require_role` and `require_writer`
  sites and the behaviour unification (W2 and W3, part 2); the "admission
  order" paragraph in the team access contract.
- Out of scope: New roles, new endpoints, wire format changes beyond the
  coordination error codes named below, graph/semantic worker semantics other
  than their admission statement, and any change to the preparation snapshot
  rules of the graph concurrency contract.
- Blockers: None. The owner decided the design, the unified outcomes and the
  order on 2026-10-19; no open product decision remains.

## Decisions

### Module shape (`db::admission`)

- Types: `Role` (Reader, Writer, Admin; ordered), `Lock` (Shared, Exclusive),
  `Transport` (Any, Browser, Device), `Archived` (Allow, Reject carrying an
  error constructor, AdminBrowserOnly carrying an error constructor) and
  `BrainAccess` (role floor, lock, transport, archived, recheck_session).
  `READ` is Reader, Shared, Any, Allow. `WRITE` is Writer, Exclusive, Any,
  Reject(the evidence `brain_archived` error). Admin access has no default and
  requires an explicit Archived choice.
- Entry points: `brain_tx(pool, auth, brain, access)` and
  `brain_tx_as(pool, actor, device, brain, access)` for workers return a plain
  `Admitted` struct holding the transaction, the effective role and the archived
  flag (no Deref). `brain_preparation_tx` is the REPEATABLE READ read-only
  variant that matches `preparation_tx`.
- `decide(access, device, role, archived)` is pure and holds the whole policy
  after the principal and visibility checks, so a unit test covers every branch.
- Surface-specific locks (the MCP catalogue advisory lock) stay in the surface
  helper after admission; the role and archive outcomes stay in `decide`.

### Migration 070

- `recollect_admit_brain(target uuid, exclusive boolean)` returns one row
  `(role text, archived boolean)`. It is VOLATILE plpgsql SECURITY DEFINER with
  the same search path and grant as `recollect_lock_brain`: it calls
  `recollect_lock_brain` first, then reads the role and the archived flag in
  separate statements. plpgsql statements take fresh READ COMMITTED snapshots,
  so both values are read after the lock wait; a single SQL SELECT would not.
  A Brain that is deleted, invisible to the caller or lost while waiting for the
  lock yields a NULL role and maps to 404. Preparation transactions keep the
  existing `recollect_lock_brain` behaviour (no lock; exclusive is rejected).
- Opening a Brain transaction drops from six to nine round trips to one after
  authentication; the lock namespace, modes and row locks are unchanged.

### Canonical order

0. Transport check (no database). 1. Principal: account, device, browser
session (401). 2. Visibility: a Brain the caller cannot see is 404 before any
lock is taken (no existence oracle; a deleted Brain is also 404). 3. Role (403).
4. Archive fence (409). 5. Surface locks and checks stay with the surface.

### Part 1 (W0, W1): no behaviour change

- W0: add the module, the migration, `decide()` tests and
  `tests/platform/admission_matrix.rs`. The matrix is a data table of
  (principal x transport x Brain state x endpoint) to (HTTP status, error code),
  recorded against the unmodified code and committed green before any
  migration. Principals: administrator, writer, reader, non-member, disabled
  account, revoked device. Transports: browser, device. Brain states: active,
  archived, deleted. One representative read and write per family: MCP
  catalogue (H1), coordination (H2), skills (H3), workspace (H4), publication
  and evidence (H5), plus one inline `require_role` read and one
  `require_writer` site. Rows that part 2 changes are marked so the flip is an
  explicit, reviewable edit.
- W1: re-implement H1 to H5 and the `evidence.rs` clone on `brain_tx`. Each
  `Archived::Reject` carries the module's existing error constructor, so codes
  and messages stay identical (`mcp_configuration_conflict`,
  `coordination_conflict`, `workspace_conflict`, `brain_archived`). Each site
  keeps its exact lock mode (workspace exclusive always; skills administrator
  and exclusive when writing; coordination and MCP follow `write`), coordination
  keeps device-only and the browser session re-check, MCP and skills keep
  browser-only writes, MCP keeps archived Brains readable except discovery, and
  skills keep archived Brains readable only for browser administrators.
  The golden matrix stays green with no expectation change. A pg_locks test
  proves a write admission holds the same lock as before, and the semantic
  admission probe that looks for `recollect_lock_brain` text is updated if its
  sites move. Admission sites in `model_gateway`, `manual_intent` and the
  semantic and graph queues move only if no concurrent change collides.

### Part 2 (W2, W3)

- W2: about 110 inline sites move mechanically. `require_role(b, false)` becomes
  READ; `require_role(b, true)` becomes Admin, Exclusive, Archived::Allow;
  `require_writer` becomes WRITE; `preparation_tx` plus a role check becomes the
  preparation variant. The 23 `auth.tx` calls without a Brain check stay.
- W3: one `brain_archived` code everywhere with order 404, 403, 409, so a
  non-admin member of an archived Brain gets 403 rather than 409 at the MCP
  configuration, private-runner, credential and workspace sites and at
  publication. Coordination `forbidden` for a missing device becomes
  `device_required` with a correct message, and `coordination_conflict` becomes
  `brain_archived`, including the A2A failure payload. The team access contract
  gains an "admission order" paragraph and the golden rows flip deliberately.

### Failure handling

- A database error is the existing 503 `database_unavailable`; a lost lock race
  (the Brain deleted, or the caller's grant removed, while waiting) is 404.
- No secret, token or credential value enters code, docs, fixtures or output.

## Acceptance

- Automated: `decide()` unit tests for every branch; the golden matrix green
  before and after W1 with no expectation change and again after W3 with the
  flipped rows; a pg_locks test that a write admission holds the exclusive lock
  and a read admission the shared lock; the MCP, coordination, skills,
  workspace, publication and evidence platform subsets no worse than the
  recorded baseline; `cargo fmt`, clippy as in `.github/workflows/code.yml`, and
  `./scripts/validate.sh`.
- Manual: N/A, no desktop or operator surface changes and no installed Brain is
  touched; behaviour is proven by disposable-database tests.
- Acceptance: Part 1 passes when the golden matrix is identical before and after
  the five helpers moved and each admission uses one statement after
  authentication. The slice ships when W2 and W3 are delivered, the contract
  paragraph is recorded and the closing Summary names the matrix and subset runs.
