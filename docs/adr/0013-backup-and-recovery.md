# 0013: Consistent encrypted backups and explicit recovery

Status: accepted
Date: 2026-09-26

## Decision

Extend the existing named-installation operator tooling. Drain its application
roles, take a PostgreSQL custom-format dump and consistent application files,
and encrypt the bundle with age. Use OpenSSH SFTP for an explicitly configured
remote destination. Reuse those maintained implementations; introduce no custom
cryptography, content-addressed identity or product-format version handshake.

Restore only into an explicitly named empty installation. Hold its application
roles behind a durable recovery marker until archive, schema, artifacts and the
latest independently retained deletion journal have been reconciled. Restore
canonical PostgreSQL state and files; rebuild Neo4j projections from eligible
canonical inputs. A database rollback cannot establish whether a previously
queued tool or model request subsequently executed, so restored outstanding
attempts become explicitly uncertain and are never blindly replayed.

When remote journal replication is configured, the existing privacy writer must
receive a durable remote acknowledgement before marking erasure complete. Keep
encrypted deletion entries separate from rotating data backups. The runtime
needs only the public encryption recipient and restricted SFTP access; the
decryption identity remains an operator recovery input. Ordinary local operation
does not require remote storage or Vault.

## Why

The accepted [stack](../foundation/techstack.md#graph-computation-and-recovery)
requires encrypted off-machine recovery from PostgreSQL and retained artifacts.
A live filesystem copy cannot provide that consistency. Reusing the installation
boundary prevents a restore from overwriting another project or silently switching
database engines. The existing deletion journal already carries opaque identity
fences and startup reconciliation; extending its durability preserves that single
authority instead of creating another erasure engine.

## Consequences

Backups have an explicit checkpoint, retention deadline and measured recovery
scope. The daily-backup target is a maximum 24-hour ordinary-data recovery point
when the operator schedule is functioning. Mutations after the recovered checkpoint
are ordinary recovery-point loss; this is not zero-loss database replication.
Rejected-value rules retained at the checkpoint govern all restored/replayed
writes. A newer retained deletion journal remains mandatory independently of that
checkpoint and prevents post-backup erasure resurrection.

Remote backup configuration is explicit and optional. A local encrypted snapshot
cannot be reported as an off-machine backup. Missing keys, unknown host keys,
unavailable journal storage, incompatible schema or partial restore leave a visible
failure/held state. No restore overwrites live data, replays an external effect,
revokes a Vault token/lease or silently enables a provider. Rollback is restoration
of a compatible checkpoint into a fresh installation, never reverse migrations.

The [recovery contract](../contracts/operations-recovery-drills.md) defines the
operator interface, failure behavior and acceptance. The full-product goal
authorizes these routine choices within the accepted foundations.

## Supersession

N/A: this completes installation's backup/recovery successor. Existing privacy,
memory authority, graph, runtime and credential contracts remain authoritative.
