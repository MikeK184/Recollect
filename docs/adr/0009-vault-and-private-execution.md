# 0009: Vault delivery and registered private execution

Status: accepted

## Decision

Extend the shared MCP executor, coordinator and receipt outbox. A private runner
is a Brain-owned registration bound to an existing paired device. A Brain admin
registers their own active device; its account must retain current Brain admin
authority. Other principals use its connections through their independent profile
grants. The runner uses its paired credential and outbound requests only. Caller
and runner identities remain separate. Use canonical `private:UUID` references.

Use HashiCorp Vault Proxy auto-authentication behind a private Unix socket.
Vault is optional per connection; environment, OS-store and anonymous connections
continue independently, including in mixed installations.
The operator configures its dedicated identity, socket and least-privilege paths.
Recollect never implements Vault login or stores its authentication tokens.
Omit proxy caching, which could independently renew connector leases, and do not
use Vault Agent's unconditional child restart for managed MCP processes.

Support coherent KV v2 field reads and leased credentials from approved GET paths.
Refresh static values on each authorized operation; unchanged values retain
compatibility. Dynamic leases are operation-owned and stay in memory. A fresh
coordinator permission check precedes acquisition and every renewal. Renewal
extends validity without changing credential generation. Rotation drains old
instances while active operations retain their own connection until completion
or bounded expiry.

## Why

The user authorized all first-usable slices and routine decisions. Accepted
foundations require Vault and authenticated private execution. The
[contract](../contracts/mcp-vault-and-private-runners.md) resolves behavior and
the [mapping](../mappings/mcp-vault-private-2026-09-22.md) records official and
Cognee/Atlas evidence. Existing Vault mechanisms already implement workload
authentication. Narrow lease API use lets Recollect enforce its application
grants and operation lifetime. Existing pairing avoids another enrollment secret.

## Consequences

Registrations have current authority/RLS, exact device bindings and audit. A
private runner is a trusted host receiving its Brain's authorized arguments and
results. Its credential retains the ordinary paired account's permissions; use a
dedicated runner account/device when separate identity is needed. This is not a
host sandbox. Vault is optional for environment/OS-store installations. Unix
socket delivery matches supported managed-stdio hosts; Windows Vault is deferred.
Expiry can leave a dispatched operation unknown. The user's explicit instruction
forbids Vault token revocation and indirect destructive cleanup. Recollect stops
renewal and lets issued leases expire; it does not call Vault revoke APIs or
delete the test namespace/auth mount. Values, lease IDs, auth tokens and raw Vault
responses never enter model context, call metadata, application logs or captures.

## Supersession

Extends ADR 0008 without changing its SDK, no-replay, ownership or no-hash posture.
