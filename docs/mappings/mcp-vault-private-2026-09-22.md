# Vault and private-runner interface evidence

Observed: 2026-09-22
Confidence: official interfaces, source review, actual Vault/Proxy/PostgreSQL and
MCP calls, focused workspace checks and normal migration/desktop preservation.

## Official interfaces

Context7 `/websites/developer_hashicorp_vault` confirms Proxy auto-auth, template
renewal, PostgreSQL dynamic credentials and explicit lease renewal. The
[Proxy API](https://developer.hashicorp.com/vault/docs/agent-and-proxy/proxy/apiproxy)
supports forced auto-auth tokens and Unix listeners. Omit cache entirely; even an
empty cache stanza enables it. The guessed `proxy/api-proxy` URL failed;
`proxy/apiproxy` is current. The
[lease API](https://developer.hashicorp.com/vault/api-docs/system/leases)
separates secret renewal from token authentication. The
[KV v2 API](https://developer.hashicorp.com/vault/api-docs/secret/kv/kv-v2)
returns coherent fields in `data.data`, without a renewable secret lease.

[Templates](https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent/template)
renew independently; [supervision](https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent/process-supervisor)
can restart children independently of Recollect operations. Neither provides
application-grant-aware renewal/draining by itself. Recollect uses the existing
Proxy for authentication and its executor's fresh authorization for secret renewal.
The [PostgreSQL plugin](https://developer.hashicorp.com/vault/docs/secrets/databases/postgresql)
issues database roles/passwords with TTL and renews their validity. It is reused,
not reimplemented. Actual login to the owned database verifies returned credentials.

Context7 `/seanmonstar/reqwest` and installed 0.13.5 source establish Unix socket
transport, disabled redirects/proxies and `retry(never())`. Response/body handling
has an outer three-second deadline and 256 KiB bound. The native CLI/server is
Vault 2.0.3; the user's Enterprise server reports 2.0.3+ent.hsm.

## Requested reference comparison

The user-requested read-only reference agent inspected clean Cognee `c0d18c8` and
Atlas `7eca7f7`. No Vault lease manager or private executor was found in bounded
searches. Cognee `api/v1/serve/serve.py:190` persists rotated OAuth refresh values
before downstream validation. `github/app_auth.py:97` mints temporary tokens,
while `github/sync.py:98` discards expiry. Integration encryption-key rotation and
`linear/adapter.py:150` stored refresh values do not establish lease renewal.
`modules/integrations/credentials.py:84` checks ownership; persistence is mocked in
its unit proof. `serve/cloud_client.py` uses a singleton proxy rather than a fenced
private executor. Both reference checkouts remain unchanged.

Atlas scope/provider/recovery patterns supply invariants, not verified Vault
adapters. Its MARM cancellation account was not reproduced here. Recollect reuses
`LocalCoordinator`, `Executor`, `RuntimeManager` and the receipt outbox. Related
fields share one fetched response. Dynamic validity changes independently of
credential generation; static rotation drains old instances while active work finishes.

## Enterprise boundary and the no-revocation instruction

The user explicitly authorized a new namespace on the existing Enterprise cluster.
Only `recollect-dev-20260922-dd66f852/` (namespace ID `beqTP`) was provisioned, with
an ownership record in `.cache/vault-enterprise-proof/boundary.json`. Bootstrap
uses the inherited operator credential with an exact namespace. Runtime Proxy
receives only dedicated AppRole files; its process does not inherit the root token.
KV and AppRole setup, coherent reads, non-root policy lookup and denied mount
administration were verified. Root mount metadata remained unchanged.

The user explicitly forbids revoking any Vault token. There are no token/lease
revoke calls or namespace/auth-mount deletion in the implementation or fixtures.
Policies explicitly deny revoke paths. Operation end stops renewal; natural issuer
TTL handles expiry. The namespace and its setup remain in place. Environment,
OS-store and anonymous connections remain independent optional alternatives.

A database engine was added only within this namespace. Its attempt to connect
to the owned Mac test database failed with `no route to host`; Enterprise dynamic
issuance/renewal is not claimed. Real dynamic proof instead uses isolated local
Vault/Proxy and PostgreSQL. New reproducible fixtures bind to loopback and record
exact process/container ownership. No customer database or paid model was used.

One fixture startup initially omitted Vault's `-dev-no-store-token` flag. Vault
wrote that generated local token to `~/.vault-token`. This was detected and the
helper was restored to the existing Enterprise token from the environment; a
fresh helper-based lookup verified root authentication. The previous helper bytes
were not captured, so exact historical file identity is not claimed. No token was
revoked. The fixture now always uses `-dev-no-store-token`; a subsequent real
fixture startup left the restored helper unchanged. Evidence:
`.cache/vault-enterprise-proof/helper-restoration.json` and
`.cache/mcp-vault-fixture-start-proof.log`. Production uses Proxy HTTP, never Vault
CLI token helpers. The dev launcher and native helper exclude `VAULT_TOKEN` from
runtime child environments; this is not used as a substitute for the no-revoke rule.

## Implementation and focused proof

- Additive migration 023 provides private registrations, Brain/device binding,
  current host eligibility, metadata-only lease reads and exact private queues.
  API/browser writes are idempotent and revision checked. Wrong devices, Brain,
  unregistered UUIDs, missing Use, disabled registration, lost host admin and
  revoked Recollect devices are denied. Direct RLS has permitted/denied controls.
- The shared executor rechecks permission before acquisition and every renewal.
  Vault values, IDs and raw failures remain on the executor. Static values are
  refetched without stale fallback; leased credentials belong to one operation.
  Renewals stop before receipt persistence/retries. Native logs expose only static
  runtime warning codes, excluding raw SDK/HTTP diagnostics.
- `.cache/mcp-vault-reproducible-proof.log`: three real-service tests pass in
  27.33 seconds using the versioned fixture launcher/test script. Enterprise KV
  reaches an actual MCP process and is redacted. Local static rotation preserves
  overlapping old/new calls and coherent fields. Dynamic renewal keeps the same
  database credentials valid beyond the original TTL; ending or denying renewal
  leads to natural expiry and rejected database login. No tool is replayed.
- `.cache/mcp-vault-boundaries.log`: six runtime unit/boundary tests pass; three
  real-service cases are explicitly ignored there and verified separately. Unix
  proxy failures cover non-renewable and denied renewal, timeout, redirect,
  oversized/invalid values, bounded leases, no retry/stale fallback, and optional
  environment/anonymous paths without a working Vault socket.
- `.cache/mcp-private-coordinator-final.log`: 17 real database/API/SDK cases pass
  in 24.69 seconds. Includes private other-caller execution, executor receipt
  recovery and an interrupted provider effect resolved by an approved receipt
  tool; each tool runs exactly once. Original uncertainty remains visible.
- `.cache/mcp-private-native-proof.log`: actual paired native private stdio and
  Streamable HTTP calls pass in 151.99 seconds, including OS-store startup,
  exact registration and shutdown. HTTP host remains running. There is no fallback.
- Two existing desktop catalogue/permission flows pass. The new registration,
  exact private selector, stale rename and disable flow passes in 13.9 seconds
  after correcting a required-label test locator. Screenshot
  `.cache/ui-mcp-private-desktop.png` was visually inspected. No mobile work.

## Closeout boundary

The 20 workspace tests and Clippy pass in
`.cache/mcp-private-workspace-final.log` and `.cache/mcp-private-clippy-final.log`.
Nine shared SDK transport/process cases pass in
`.cache/mcp-private-transports-final.log`. The dotted-segment Vault path
refinement passes its focused boundary check. API generation contains 155
operations; TypeScript and the desktop build pass. Governance checks pass all
32 cases. These checks supplement the actual service proof above.

Normal migration 023 is applied and slice 24 is delivered locally. The frozen
pre-023 dump passes `pg_restore --list`; exact before/after inventories preserve
seven Brain identities, 33 model requests, zero active jobs and the user's
`test` profile including its revision, grants and connections. The normal runtime
has no calls, instances or private registrations. Readiness and two successive
central lease samples pass. API/worker environments exclude the inherited root
token; the restored Enterprise helper still matches that existing credential.
Actual desktop inspection and SWEG recall return one item, 1,630 bytes in 62 ms,
with no browser errors, new model requests or customer file reads. Evidence and
the visually inspected screenshot are in `.cache/mcp-vault-normal-proof/`.
No fixture catalogue or connection was imported into normal data. Changes remain
uncommitted; no customer service deployment is claimed. The accepted
[ADR](../adr/0009-vault-and-private-execution.md) and
[contract](../contracts/mcp-vault-and-private-runners.md) govern completion.
