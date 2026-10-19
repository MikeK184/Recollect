# 0024: Local password hashing and strict credential-file reads

Status: accepted
Date: 2026-10-19

## Decision

The repository owner authorized on 2026-10-19 that invited local accounts follow
the foundation's Argon2id requirement instead of the development exception in
[ADR 0003](0003-product-runtime.md). This supersedes only the part of ADR 0003
that allowed storing and comparing local account passwords without hashing.

Passwords of invited local accounts are stored only as Argon2id PHC strings
(version 19, memory 19,456 KiB, 2 passes, parallelism 1, a fresh random 16-byte
salt per entry) in the existing credential file, `RECOLLECT_CREDENTIAL_FILE`.
The file keeps its JSON shape (random credential ID to value), its 0600 mode and
its atomic temp-file, fsync and rename replacement. PostgreSQL still stores only
the random credential reference. Hashing and verification run on blocking
threads with a small fixed concurrency bound; the stored parameters are authoritative
when verifying, so a future cost change needs no format change.

The environment-backed installation owner password is not stored in the file. It
keeps its constant-time comparison with the environment value and is out of scope.
Credential values that are not login passwords, namely MCP `local_file` provider
secrets that are delivered to upstream servers, cannot be hashed. They remain
private 0600 files under the MCP credential contract.

Entries written by earlier versions are plaintext. The server converts every
legacy entry to Argon2id during startup before it serves, with one atomic file
replacement and without logging any value (only the number converted). A legacy
entry met at login despite a skipped or failed startup conversion is accepted
when it matches exactly and is upgraded then. After one successful start no
plaintext remains.

Reading the credential file is as strict as the MCP runtime's secret-file check:
the path must be a regular file, not a symlink, mode without group or other bits,
owned by the server's effective user, and at most 8 MiB. Otherwise the read fails
closed and login or enrolment returns 503 `credentials_unavailable`; the
environment-backed owner stays the recovery path.

## Why

[Techstack](../foundation/techstack.md) requires Argon2id password hashing. ADR 0003
relaxed it for local development speed, but shared HTTPS installation
([ADR 0011](0011-local-and-shared-installation.md)) now exists and nothing stopped
plaintext there. A copied or backed-up credential file should not disclose
teammates' passwords, and a file with loose permissions or a symlink should not
be trusted. The OWASP baseline parameters keep a login near tens of milliseconds
and about 19 MiB transient memory.

## Consequences

Each teammate login costs one Argon2id verification; the existing per-address and
per-name throttle still bounds attempts. A local sign-in for an unknown, disabled or
unprovisioned account, and for the owner, also performs one verification against
a dummy hash so response timing does not reveal which usernames exist. A
previous server version cannot verify hashed entries; rolling back requires the
owner to use Reset sign-in for affected teammates. Restoring an old backup of the
file re-triggers the automatic conversion. Recovery from a
corrupt or insecure file stays with the operator: fix the file mode or restore a
copy, then restart.

## Supersession

Supersedes the password-hashing exception in
[ADR 0003](0003-product-runtime.md) (the "do not hash" sentence for local account
credentials). The rest of ADR 0003, including its other dependency and version
relaxations and the environment-compared owner password, remains accepted.
