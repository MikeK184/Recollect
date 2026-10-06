# Optional Vault credentials and private runners

## Prerequisites

Use migration 023, a paired companion and the approved
[MCP catalogue/runtime](mcp-runtime.md). The
[contract](../contracts/mcp-vault-and-private-runners.md) governs this extension.
Vault is optional per connection. A Brain and runner can mix environment,
OS-store, Vault and anonymous connections. Choosing no credential alias needs no
credential file. Existing environment/OS-store bindings need no Vault service.

## Register a private runner

In the Brain's MCP panel, choose **Register private runner**, name it and select
your own active paired device. You must remain a Brain administrator. Registration
alone shows **Offline**; **Connected** requires a current runner heartbeat.
Other users need their own explicit profile **Use** grant. They do not receive
the host's account or device credential.

On that registered device, use its paired endpoint/profile and an owned receipt
directory. Install `recollect-mcp-runner` beside `recollect-agent`:

```sh
cargo build -p recollect-agent
RECOLLECT_URL=http://127.0.0.1:8787 RECOLLECT_DEVICE_PROFILE=private-host \
  target/debug/recollect-agent private-runner RUNNER_UUID /path/to/private/receipts
```

Set connection placement to **private** and select that exact Brain registration.
Offline calls remain queued until their queue deadline; another runner does not
take over. Rename/disable through **Edit private runner**. A device change needs
a new registration. Stale edits require reopening the form. Lost host authority
prevents new work and renewal; active uncertainty uses the existing receipt-only
recovery and reconciliation controls. Stopping a runner never revokes Vault tokens.

## Configure optional Vault delivery

Run an operator-owned Vault Proxy with a dedicated scoped identity and private
Unix socket. The socket and immediate directory must belong to the executor's
user and exclude group/other access. Keep the socket path short enough for the
host's Unix socket limit. Namespace belongs in Proxy configuration. Example:

```hcl
vault {
  address = "https://vault.example.internal:8200"
  namespace = "recollect/"
  retry { num_retries = 0 }
}
auto_auth {
  method "approle" {
    namespace = "recollect/"
    mount_path = "auth/approle"
    config = {
      role_id_file_path = "/private/recollect/role-id"
      secret_id_file_path = "/private/recollect/secret-id"
      remove_secret_id_file_after_reading = false
    }
  }
}
api_proxy {
  use_auto_auth_token = "force"
  prepend_configured_namespace = true
}
listener "unix" {
  address = "/private/recollect/proxy.sock"
  socket_mode = "0600"
  tls_disable = true
}
```

Omit `cache` entirely. It could renew secrets independently of Recollect's current
permissions. Proxy auto-auth manages its own workload identity. Recollect never
logs in to Vault or uses its CLI token helper. The tested Vault 2.0.3 socket has
owner-only mode 0700 despite the 0600 setting; both satisfy the ownership boundary.

Point `RECOLLECT_MCP_CREDENTIALS_FILE` on the selected executor at a JSON file of
references, with a binding for the connection UUID, approved alias and exact
runner. Vault/private bindings contain no UI-entered values; models cannot
provision credentials. The separate central development-file owner workflow is
documented in [MCP runtime](mcp-runtime.md#browser-managed-connector-setup). Example:

```json
{
  "vault_sources": {
    "connector-key": {
      "socket": "/private/recollect/proxy.sock",
      "path": "kv/data/connector",
      "kind": "kv_v2"
    }
  },
  "bindings": [{
    "connection_id": "CONNECTION_UUID",
    "alias": "approved-read-alias",
    "runner_reference": "private:RUNNER_UUID",
    "headers": {
      "authorization": {
        "source": {"provider": "vault", "source": "connector-key", "field": "api_key"},
        "prefix": "Bearer "
      }
    }
  }]
}
```

For stdio, use `environment` destinations instead of HTTP `headers`. Sources can
also be `environment` or `os_store`, as described in the main runtime runbook.
An unused Vault source does not affect those alternatives. Reserve the ambient
`VAULT_TOKEN` for operator bootstrap; use a separately named environment reference
if a connector itself requires its own scoped Vault token.

For leased credentials select `kind: "leased"`, an approved secret GET path such
as `database/creds/runner`, and top-level string fields such as `username` and
`password`. Related fields from one named source use one response. Scope Proxy's
policy to those reads and `sys/leases/renew` for those lease-ID prefixes. Do not
grant revoke paths. Recollect calls no token or lease revoke endpoint.

Static secrets refresh per operation; changed values drain old instances while
active work finishes. A Vault failure has no stale fallback. Dynamic leases are
owned by one operation and are renewed near half their remaining TTL after fresh
Recollect permission checks. Startup/calls cannot outlive the last known validity.
Renewal denial/failure stops renewal; a dispatched expiry stays **Unknown**.
Operation end stops renewal before receipt uploads. Issuer TTL provides natural
expiry; this does not claim immediate invalidation of already issued credentials.

## Verification and recovery

The [dated mapping](../mappings/mcp-vault-private-2026-09-22.md) distinguishes
Enterprise KV/Proxy proof from local dynamic issuance/renewal proof. The Enterprise
cluster cannot route back to the Mac fixture database. Its authorized namespace
and auth setup must remain in place; never revoke tokens or delete it as cleanup.

For a new isolated local fixture, install Vault CLI and use the repository Docker
wrapper. This creates only owned loopback services and ignored private files:

```sh
./scripts/start-vault-fixture.py
```

It prints a fixture directory and a test command to run in another terminal:

```sh
./scripts/test-mcp-vault.sh /absolute/path/to/printed/fixture
```

The launcher uses `-dev-no-store-token` and suppresses Vault console output. It
never inherits the Enterprise root credential into Vault/Proxy processes and never
changes the CLI token helper. Runtime uses AppRole. An optional second test-script
argument selects the already authorized Enterprise fixture for a read-only KV
check. No Enterprise target is discovered or bootstrapped automatically.

Keep the fixture services/namespace in place when finished. Their exact ownership
is recorded in `boundary.json`; no automatic token/lease revocation, auth disablement
or namespace deletion is provided. Production Proxy policy/configuration belongs
to its operator. Environment removal alone does not make a Vault cleanup command
safe; the implementation does not issue those commands.

`credential_vault_denied`, `credential_vault_unavailable`,
`credential_vault_response_invalid`, `credential_proxy_invalid`,
`credential_lease_invalid`, `credential_renewal_denied` and `credential_expired`
are safe diagnostics. Correct the configured source, role or permission; inspect
unknown outcomes through receipt/evidence reconciliation before submitting a new
effect. Never repeat a dispatched tool merely to acquire new credentials.
