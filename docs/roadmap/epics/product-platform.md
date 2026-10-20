# Product Platform and Access

Status: complete

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
[archived pack](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/platform-bootstrap.md). Durable work is also
delivered in its [archived pack](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/platform-durable-work.md).
Team access is delivered in its [archived pack](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/platform-team-access.md).
Device pairing is delivered in its [archived pack](https://github.com/MikeK184/Recollect/blob/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive/platform-device-pairing.md).

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
| `platform-brain-deletion-fixture` | shipped | adr-backed, contract-backed | inline: the shared Brain deletion fixture holds a Brain A row in every `brain_deletion_dependents` relation, so the full-closure proof (counter agreement, tombstone, content-free retained rows, per-record erasure) runs again, and the API test expects the contract's pending work and pending artifacts; `brain_deletion` and `brain_deletion_api` on a disposable PostgreSQL, `./scripts/validate.sh` | Test-only repair; no product, schema or contract change and no assertion relaxed. The fixture now seeds the 48 relations that 047-067 added to the dependent list (maintenance cursors, managed skills, coordination, mapping producer, origin, stage and coverage, source inputs, organization, shadow, topic, alias, retrieval feedback, structured field and evidence intent rows). Its model-input fence moved to a superseded version without derivatives: since 049 a fence follows exact source ancestry, and with the excerpt re-parented onto that version the old fence cancelled the semantic batch's job (`pending_work` 4); all five seeded jobs are pending again, as both tests expect. The API expectation `pending_artifacts` follows the SQL-level test to 2, which 18a8ba7 changed when the pending excerpt intent joined the fixture. Proven 2026-10-20 on a disposable PostgreSQL 17: all four tests pass. Known gap, not encoded: an evidence artifact intent whose canonical version never committed is absent from the Brain-wide closure (only the unused `k='brain'` branch of 067 names it), so the fixture's evidence intent is a committed one; the cleanup fix is separate work |
| `desktop-knowledge-surface` | shipped | adr-backed, contract-backed | archived | Tiered sidebar, merged Memory/Sources/Graph/Repositories surface, shared lineage inspector, graph chrome and list density |
| `platform-local-credential-hardening` | shipped | adr-backed, contract-backed | archived | Invited local passwords stored only as Argon2id hashes (ADR 0024), legacy plaintext converted at startup or sign-in and the credential file read fail-closed (503); proven by 5 unit tests, a new platform test, a real `serve` startup run, and unchanged auth/team/MCP platform subsets (30 pass, the same 13 unrelated failures as before the change) |
| `platform-brain-admission` | shipped | contract-backed | archived | One typed Brain admission (`db::admission`, migration 070): every Brain-scoped request or worker transaction admits in one statement in the order transport, principal 401, visibility 404, role 403, archive 409 `brain_archived`, with lock and statement timeouts as access options; the five helpers, all 159 `require_role`/`require_writer` sites and 15 of 17 standalone `lock_brain` calls moved, and 41 golden-matrix cells flipped on purpose; proven by the 363-cell status matrix (green and unchanged after the move, then flipped), pg_locks equivalence, one-statement and timeout platform tests, 9 admission unit tests, the full platform run (314 of 335 pass at HEAD; the 21 failures also fail on main) and the unchanged 40-test MCP, coordination, skills, workspace, publication and evidence subset |
| `platform-oidc-shared-tls` | shipped | contract-backed | inline: OIDC discovery and token calls verify the provider with the same roots as every other server call (system store, plus `SSL_CERT_FILE`/`SSL_CERT_DIR` on Linux) through the shared five-second, no-redirect client; the image's trust store is provable without configuration, a database or a key; the [team-access](../../contracts/platform-team-access.md) and [operations](../../contracts/operations-local-and-shared.md) contracts state both; Dex `scripts/test-oidc.sh`, `tls-probe` unit tests, a Linux CA-signed listener test and one reqwest in `cargo tree -d` | `openidconnect` now runs without its bundled reqwest 0.12 (and its separate webpki root set) through a 1 MiB-bounded `AsyncHttpClient` over `AppState::http`, and `recollect-server tls-probe [URL]` prints one JSON line for the trust path (the default URL answers `{"tls":"ok","stage":"http","status":401}`); proven by `oidc_live` 1/1 through `scripts/test-oidc.sh` before and after, 5 probe unit tests, and the Linux listener test in a rust:1.94-slim-bookworm container (trusted CA ok/401, a different CA fails at the handshake, `SSL_CERT_FILE=/dev/null` fails at roots, no override fails; a debian image without `ca-certificates` fails at roots); `scripts/test-installation-runtime.py` now runs the probe in the API container but was not run (no proof installation), and no external Entra or private-CA issuer was exercised |
| `platform-brain-deletion-scale` | shipped | adr-backed, contract-backed | inline: a realistically sized Brain previews and deletes through the API under the pool's default 10 s statement timeout, the closure document and counter equal the iterated per-target union on the same data, tombstone, journal and visibility rules unchanged, both handlers bounded at 300 s with 10 s lock waits ([contract](../../contracts/platform-brain-deletion.md#closure-computation)); `brain_deletion` equivalence, scale and gate tests, the API large-Brain test, the deletion, erasure and replay platform subset, `./scripts/validate.sh` | Brain-wide closure computed once per Brain (migration 074, identical document and counter) and the apply chain's derived-state row triggers skipped for the Brain being removed (migration 075); no schema, journal or manifest change, so no pack. Before: 960 targets took 187 s to preview and 570 sources with 1,137 claims exceeded the 10 s timeout. After, 3,000 sources and 5,000 claims (8,000 targets) preview in 0.3 s and delete in 6.0 s at the SQL layer, 0.34 s and 6.7 s through the API. Proven 2026-10-20 on a disposable PostgreSQL 17 and Neo4j 2026.08: the closure equals `recollect_iterated_brain_closure` on the rich fixture, the control Brain, a generated Brain reaching every closure class and an empty Brain; a caller-set removal marker never opens the trigger skip; the 91-test platform subset passes (deletion, erasure, retention, privacy and replay, including populated Brain deletion and older-backup replay). `brain_deletion_full_closure` ("fixture gap") and two `brain_deletion_api` expectations (`pending_work` 4 vs 5, `pending_artifacts` 2 vs 1) fail identically on unmodified HEAD and are left for a separate fix |
| `platform-brain-deletion-evidence-intents` | shipped | adr-backed, contract-backed | inline: deleting a Brain names the file of every evidence artifact intent, pending or already swept, in the journal manifest, so the cleanup queue removes it and restore replay removes an older copy; the preview counts those files, an upload that starts after the preview makes `DELETE` a changed preview, another Brain's pending upload stays; the closure equals the iterated per-target union plus exactly those intents ([contract](../../contracts/platform-brain-deletion.md#closure-computation)); `brain_deletion_api_removes_pending_evidence_artifacts`, `brain_closure_matches_iterated_reference`, the deletion, erasure, retention, privacy and replay platform subset, `./scripts/validate.sh` | Migration 076 adds the Brain's `evidence_artifact_intents` to the closure's artifact set. 067 had named them only in a `k='brain'` branch of the per-target closure that the Brain closure (029, 074) never passes, and `recollect_brain_remove` deleted the intent rows, so every uncommitted upload of a deleted Brain stayed on disk: confirmed on 029 and 074, where the deletion queued no `privacy_artifacts` row and both files survived `privacy_journal::run_once`. The user chose on 2026-10-20 to name every intent, pending or swept. The counter changes only for a Brain holding intent rows and the manifest format is unchanged, so no pack. The 039 excerpt intents were already covered through their committed parent versions, and the other unused `k='brain'` branches (045, 054, 056, 057, 058, 060, 062) name rows only, which the apply chain and `recollect_brain_remove` delete. Proven 2026-10-20 on a disposable PostgreSQL 17 and Neo4j 2026.08 with GDS: the new API test (a Brain with content, a Brain whose only upload failed, a swept intent, a late upload, an excerpt intent and a control Brain) fails before 076 and passes after, with both deletion requests `complete`; the equivalence test passes with the exception scoped to `artifacts` and to Brains holding intents; the scale and gate tests pass; 89 of 89 runnable platform subset tests pass (`oidc_live` and the two SFTP recovery cases need fixtures `scripts/test-platform.sh` skips). `brain_deletion_full_closure` and the two older `brain_deletion_api` expectations fail as on 35102f1; with the separate `platform-brain-deletion-fixture` repair (5319f2b) applied on top, all nine `brain_deletion` and `brain_deletion_api` tests pass |

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

Accounts, Brain permissions, auditable durable commands, device identity and canonical Brain deletion are delivered. The original platform slices have no unfinished acceptance in this epic. New automatic mapping must extend existing isolation/deletion behavior under the graph organization pack. The `platform-brain-admission` refactor (2026-10-19) is delivered: one typed admission serves every Brain-scoped request and worker transaction in the order recorded in the [team access contract](../../contracts/platform-team-access.md#admission-order), every surface answers an archived Brain with 409 `brain_archived` after the role check, and no role, grant or isolation rule changed. The open-transaction helpers `db::require_role` and `db::require_writer` remain as thin wrappers for platform tests only. Brain deletion scales to large Brains since 2026-10-20 (`platform-brain-deletion-scale`): the closure is computed once per Brain with an unchanged counter, and the delete skips derived-state upkeep for rows it removes. Since `platform-brain-deletion-evidence-intents` the deletion also removes the files of uploads whose source version never committed.

See the [current roadmap](index.md) for priorities and the
[active packs](../execution/active/README.md) for exact remaining acceptance.

## Delivery evidence

[Shipped packs](https://github.com/MikeK184/Recollect/tree/873156174f421da0574952a5e533fb6a6133fab0/docs/roadmap/execution/archive) were removed from the tree on 2026-10-19 and remain in Git history.
The superseded chronological closeouts and original research references remain
in the [pre-cleanup epic record](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/docs/roadmap/epics/product-platform.md).
Dated test/deployment observations are not a current runtime health claim.
