# Local credential hardening

Status: in-progress
Owning epic: `docs/roadmap/epics/product-platform.md`

## Summary

- Goal: Invited local account passwords are stored only as Argon2id hashes, any
  legacy plaintext entry is converted automatically, and the credential file is
  read with the same strictness as the MCP runtime's secret file.
- Non-goals: Hashing the environment-backed owner password (it is never stored),
  hashing MCP `local_file` provider secrets (they must be sent upstream), changing
  the file location or JSON shape, OIDC, new throttling or a rehash-on-parameter-change policy.

## Governing Sources

- [ADR 0024: Local password hashing](../../../adr/0024-local-password-hashing.md),
  which supersedes the no-hashing part of [ADR 0003](../../../adr/0003-product-runtime.md).
- [ADR 0011: Personal and shared installation](../../../adr/0011-local-and-shared-installation.md).
- [Team access contract](../../../contracts/platform-team-access.md) and
  [bootstrap contract](../../../contracts/platform-bootstrap.md).
- [Operations contract](../../../contracts/operations-local-and-shared.md) and
  [MCP credentials contract](../../../contracts/mcp-runtime-and-credentials.md).
- [Techstack identity requirement](../../../foundation/techstack.md) and the
  [owning epic](../../epics/product-platform.md).

## Scope

- In scope: `crates/server/src/credentials.rs` (hash on write, verify on read,
  legacy conversion, strict read), login and enrolment callers, a startup
  conversion before `serve` listens, the `argon2` dependency (optimized in the
  dev profile so tests stay fast), tests, the `scripts/setup-local.sh` conflict
  check (a hashed runtime copy of the same credential IDs is not a conflict), and
  the contract, runbook and ADR text that described plaintext storage.
- Out of scope: `crates/mcp-runtime` behavior, the MCP secrets file, Compose or
  installer ownership changes, an operator CLI for conversion, parameter tuning.
- Blockers: None; the repository owner authorized the design on 2026-10-19.

## Decisions

- Storage: the existing 0600 JSON file maps credential ID to an Argon2id PHC
  string (`$argon2id$v=19$m=19456,t=2,p=1$salt$hash`, 16-byte random salt,
  32-byte output). Explicit parameters in code, not crate defaults. Verification
  uses the parameters embedded in the stored string.
- Write path: enrolment hashes on a blocking thread, then writes the whole file by
  the existing 0600 temp, fsync, rename and directory fsync path under the same
  process-wide lock. A failed database transaction can still leave an unused
  hashed entry, never a partial activation.
- Read path: use the MCP runtime's strict reader (no symlink, regular file, mode
  with no group/other bits, owned by the effective user, at most 8 MiB, valid
  JSON). A missing file is unavailable at login, and missing-allowed only when
  enrolment or startup creates or checks the file. Any failure maps to 503
  `credentials_unavailable` without echoing a path or value.
- Verification: the file lock is held only to read the entry, not during hashing,
  so logins do not serialize. At most four Argon2 operations run at once; others
  wait. The legacy plaintext comparison uses a constant-time comparison.
- Timing: a local login for an unknown, disabled, unprovisioned or owner account
  performs one verification against a fixed dummy hash so latency does not reveal
  account existence. The throttle (100 per address, 10 per address and name per
  minute) stays the only attempt bound.
- Migration: at `serve` startup, before binding the listener, convert every
  non-PHC entry to Argon2id and replace the file atomically (0600, fsync, rename).
  Log only the converted count. A missing file converts nothing. An invalid or
  insecure file is logged as a warning without values and the server still
  starts; teammate login then returns 503 while the owner can sign in.
  A matching legacy entry met at login is upgraded in place under the lock; if
  that write fails, login still succeeds and the next start retries.
- Detection: an entry that parses as an Argon2id PHC string is a hash; anything
  else is legacy plaintext. (A legacy password that is itself a valid PHC string
  is not a realistic input; an owner can reset the account.)
- Documents: ADR 0024 (new) supersedes the ADR 0003 allowance; update the team
  access and operations contracts, the team-access and installation runbooks, and
  the ADR index. The shipped slice summary records validation.
- Rollback: an older server cannot verify hashes; the owner uses Reset sign-in.
  Restoring an older plaintext backup is converted again at the next start.

## Acceptance

- Automated: platform tests prove stored values are PHC strings after enrolment,
  a legacy plaintext file is converted at startup and the teammate can still sign
  in, a wrong password is rejected, a 0644 file and a symlink return 503 at login,
  and the converted file stays 0600; unit tests cover hashing, verification,
  legacy detection and upgrade. `cargo fmt`, clippy as in the code workflow, the
  focused auth/team/credential platform subsets compared with the baseline taken
  before the change, and `./scripts/validate.sh`.
- Manual: N/A; no installed Brain, credential file or container is touched.
- Acceptance: no plaintext password remains in a file after one start; invalid,
  insecure or symlinked credential files fail closed; ADR 0024 and the affected
  contracts and runbooks match the code; the pack is deleted and the slice is
  archived with a summary naming the validation.
