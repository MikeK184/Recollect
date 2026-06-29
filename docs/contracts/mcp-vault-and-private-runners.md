# Vault credentials and private runners

Status: accepted

## Source

The full product goal authorizes these routine decisions under the
[foundation](../foundation/vision.md#mcp-coordinator-and-vault-integration),
[ADR 0009](../adr/0009-vault-and-private-execution.md),
[runtime](mcp-runtime-and-credentials.md) and
[paired identity](platform-device-pairing.md) contracts. The
[mapping](../mappings/mcp-vault-private-2026-09-22.md) is evidence only.

## Contract

### Private registration and execution

A Brain owns at most 32 private-runner registrations: UUID, Brain, name (1–120
characters), immutable paired device UUID, enabled flag, revision UUID, creator
and timestamps. Names and device bindings are unique within a Brain. A current
Brain admin with browser authentication registers their own claimed, unexpired,
unrevoked device on an open Brain. Updates rename or enable/disable; changing the
device requires a new registration. Disabled history remains. Registration is
metadata, never proof of connectivity.

`GET/POST /api/brains/{brain}/mcp/private-runners` list/create;
`PUT /api/brains/{brain}/mcp/private-runners/{id}` updates with `base_revision`.
Reuse command-idempotency headers, Brain transaction locks, revision conflicts,
RLS and append-only audit. Brain readers see metadata/current eligibility; only
admins mutate. Tokens are never returned. Cross-Brain IDs remain unavailable.

Private connections select canonical `private:UUID` in the same Brain. Enabled
new/changed connections require eligible enabled registration; an otherwise
unchanged old connection can still be disabled when its runner is unavailable.
Admission, claim, startup, credential acquisition/renewal and pre-send recheck
that registration, its exact device, active paired account and current Brain admin
role, plus the caller's existing scope/Use/frozen configuration checks.

Existing runner routes accept optional `private_runner=UUID`. It is authenticated
runner routing, never a caller tool argument. Omission remains personal-device
execution. Wrong/unregistered devices cannot register, claim, renew, complete or
update another runner's work. Queues expose only the registration's addressed
Brain calls; private execution can serve other callers, with identities kept
separate. Reuse single live epoch, 16 active calls/instances, 30-second lease,
receipt-only recovery and no central/local fallback.

`recollect-agent private-runner RUNNER_UUID OUTBOX_DIRECTORY` executes the helper
with its paired endpoint/profile. Outbox identity includes exact endpoint, device
and registration. Offline work queues until expiry; lost dispatched work becomes
unknown and a later receipt never replays it. Disable or device/account/Brain
revocation denies new calls and renewals. Existing heartbeat cancellation and
operation deadlines govern active work; issued credentials are not automatically
invalidated by application grant changes.

### Vault references and delivery

Vault is an optional provider selected per connection binding. Environment,
OS-store, Vault and anonymous connections can coexist in the same Brain and
runner. An unselected or unavailable Vault source does not prevent an environment,
OS-store or anonymous connection from running. No Vault setup is required for
those alternatives. This preserves the user's explicit 2026-09-22 clarification.

The operator credential-reference file gains `vault_sources`, at most 128 named
entries. Each specifies absolute `socket`, relative Vault API `path` and `kind`
(`kv_v2` or `leased`). Namespace is fixed in proxy configuration. Paths have at
most 512 bytes of slash-separated ASCII letters, digits, dots, hyphens and
underscores, with no `.` or `..` segments, percent
encoding, query or fragment; `sys/`, `auth/` and token endpoints are prohibited.
Models cannot choose URLs, auth values, methods, bodies or executable templates.

Destination references use `{"provider":"vault","source":"NAME","field":"FIELD"}`.
Existing connection/alias/runner binding must match. A resolution fetches each
referenced source once, at most eight sources, so related fields share one
response. Read bounded top-level string fields from `data.data` for KV v2 or
`data` for leased responses. Existing destination and 4–16,384-byte bounds apply;
there is no value coercion or fallback.

Use a dedicated operator-controlled Proxy socket and owner-only directory/socket
permissions. Configure `api_proxy.use_auto_auth_token="force"` and omit `cache`
entirely. Reqwest Unix socket transport uses no redirects, ambient proxies or
client retries, a three-second deadline and 256 KiB response bound. Only approved
reads and owned secret-lease renewal APIs are called. No Vault token or lease
revoke API is used. Raw errors/responses,
paths, tokens and lease IDs are excluded from application diagnostics.

New `POST /api/mcp/runner/credentials` takes `McpAttempt` and the same private
selector. In one authority transaction it checks exact runner/attempt, current
lease/deadline, cancellation, account/device/Brain, scope, Use, registration and
configuration. The central adapter calls the same logic. Every acquisition and
renewal requires a fresh check; none is cached. Five-second control timeout
prevents credential I/O. Revocation after a successful check cannot retract a
Vault request already started; its three-second deadline bounds that race.
Proxy workload-authentication renewal is separate from connector-secret renewal.

### Validity, rotation and draining

KV v2 is fetched per authorized call. Unchanged values retain generation; changes
drain older instances while active calls finish. Vault unavailability has no stale
fallback. Static secrets have no fabricated lease expiry/invalidation guarantee.

Leased responses require a nonempty lease ID of at most 1,024 bytes, a duration
of 1–86,400 seconds and
renewability. Each operation acquires its own lease, without sharing it across
operations or persisting it. Track monotonic validity, conservatively subtracting
the full request duration. Renew near half its remaining duration through
`sys/leases/renew`, preceded by current authorization. Only the same lease's
returned validity is accepted; unchanged bytes keep their generation. Do not renew
non-renewable leases. Startup and active calls cannot exceed credential expiry.
Expiry cancels the wait/drains the owned instance; dispatched uncertainty remains
unknown unless a reliable response arrived. Never repeat the tool for new secrets.

Renewal denial/failure stops renewal and reuse; active work can finish within its
last known validity and call deadline, subject to authority cancellation. Ending
the operation stops renewal and drains its dynamic instance. Issuer TTL provides
natural expiry, including after a crash; do not claim immediate invalidation.
Missing/invalid/unavailable/expired
credentials and denied renewal have explicit non-secret codes.

### Desktop and operation

The MCP panel lists private registrations, eligibility and last live lease,
supports admin registration/rename/disable, and provides exact private-runner
selection in connection editing. Empty devices/runners, denied authority, stale
revisions and disconnected runners are explicit. Credential configuration stays
in an operator file; UI/model APIs never edit or return values. Existing call
details show selected runner and safe failure codes. Provide tested Proxy config
and native startup runbooks. There is no per-call human queue or mobile scope.

## Acceptance

Use an owned real Vault fixture or the user's explicitly authorized Enterprise
cluster. Every new setup on that cluster requires its own dedicated namespace;
an isolated local Community fixture uses a separate owned server, since that
edition has no namespaces. Root
credentials are inherited only for bootstrap; runtime uses scoped AppRole.
Preserve existing namespaces/mounts and keep an exact non-secret ownership record
before subsequent writes. The user's further instruction prohibits revoking any
Vault token, including test tokens, and prohibits indirect cleanup through
namespace deletion/auth-mount disablement. Keep this dedicated setup for continued
work and allow natural TTL expiry. Do not depend on shell `unset`; bootstrap and
runtime credentials have explicit request/process boundaries. Use a real Proxy,
generated transient credentials and
synthetic data to prove KV rotation/coherent fields, dynamic issuance/renewal,
unchanged-byte validity extension, failed/expired/denied renewal, cleanup and
redaction. Suppress Vault startup console output, which can print dev credentials.
Do not contact other customer Vaults or use production connector credentials or
paid models. The user's 2026-09-22 namespace authorization extends the local-test
boundary specifically to this Enterprise integration setup.
Prove real private stdio and Streamable HTTP calls via authenticated production
routes; wrong device/Brain/registration, missing Use and disabled/revoked authority
must prevent dispatch with permitted controls. Exercise overlapping long work
during rotation, exact placement/no fallback, interruption and receipt recovery
without effect replay. Include direct API/RLS, desktop and native proofs, affected
runtime/workspace checks and `./scripts/validate.sh`. Inspect desktop state and
preserve normal data across additive migration; configuration is not runtime proof.

## Explicit Deferrals

Customer deployment/policies outside the authorized new test namespace, arbitrary
Vault auth/secret-engine implementations,
Agent-driven child restart, Recollect secret persistence, Windows Vault delivery,
mobile, memory/workspace tools and observation capture. Proxy supports operator
auth methods; the owned proof uses AppRole.
