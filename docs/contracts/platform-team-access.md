# Team identity and effective access

Status: accepted

## Source

[Runtime ADR](../adr/0003-product-runtime.md), [bootstrap contract](platform-bootstrap.md),
[durable-work contract](platform-durable-work.md) and the
[platform epic](../roadmap/epics/product-platform.md). The user authorizes routine
contracts for the complete product. Foundation access requirements remain in force.

## Contract

### Accounts and invitations

Only the installation owner manages accounts. There is no public registration.
`GET /api/team` lists enabled/disabled accounts and pending invitation metadata,
without credentials. `POST /api/team/invitations` reserves a unique username and
returns a single-use UUID enrollment token, valid 24 hours. The owner shares it
manually; Recollect sends no email. `DELETE /api/team/invitations/{id}` revokes an
unused invitation. Repeating a used/expired/revoked invitation returns 410.
Usernames are trimmed, case-sensitive, 1–120 characters; duplicates return 409.
No invitation implicitly grants access to a Brain.

`POST /api/auth/enroll` accepts token and a password of 8–4096 bytes, activates the
reserved account and returns a normal session. Consuming the token and activation
are atomic; concurrent acceptance creates one credential/session. Invitation URLs
use a fragment read by the browser, then remove it from browser history. Tokens
and passwords do not enter audit, logs, list responses or command receipts.

Under the no-hashing instruction, invited local credentials live in an ignored
0600 JSON file (`RECOLLECT_CREDENTIAL_FILE`, default `.data/credentials.json`).
Accounts reference random credential IDs; no password is stored in PostgreSQL.
The single local API process serializes atomic file replacement. A new credential
is durably written before the database activates it; a failed database transaction
can leave an unused credential entry, never a partially activated account.
Runtime reads detect a missing/corrupt file as 503, not a successful login.
The environment-backed owner remains the recovery path. No password hashes or
application digest/version gates are added; OIDC's library signature verification
is part of its protocol, consistent with ADR 0003.

`PATCH /api/team/accounts/{id}` enables/disables a non-owner account. Disable
immediately revokes all its sessions and denies queued publication. Re-enable
does not restore revoked sessions. The installation owner cannot be disabled.
`POST /api/team/accounts/{id}/reset` issues a fresh enrollment invitation for a
local account, invalidates its old credential reference and sessions, and disables
it until acceptance. OIDC identities use their provider's recovery instead.
Account, invitation, login enrollment, reset and access mutations are audited
atomically; installation audit is visible only to the installation owner.

### Brain authority

`GET /api/brains/{id}/access` is admin-only and reports account ID/name, enabled
state, ownership, direct role, current mapped groups, effective role and membership
deadline. Ownership, direct grants and inherited mappings are independent and
additive (`admin > writer > reader`). Disabled accounts have no effective role.
No installation-owner bypass of Brain grants exists.

`PUT /api/brains/{id}/grants/{account}` sets a direct reader/writer/admin grant;
`DELETE` removes only that grant. `PUT /api/brains/{id}/group-grants` upserts one
exact OIDC group-to-role mapping; `DELETE /api/brains/{id}/group-grants/{mapping}`
removes it. These require current Brain admin. Group mappings apply only to
explicitly enrolled OIDC principals and fresh complete membership snapshots.
Removing any source reports remaining effective access, including ownership.
`POST /api/brains/{id}/grants` resolves an enabled account by exact username and
sets the same direct grant. Group mappings retain the configured issuer as well
as the group name; claims from another issuer cannot satisfy them.

`POST /api/brains/{id}/owner` transfers ownership to an enabled existing principal.
Only the current Brain owner can transfer; the old owner retains only independently
granted roles. Contribution/audit identities remain unchanged. Team Brains can be
owned by a designated account and shared with direct/group grants; this local
delivery has no synthetic organization-owner bypass. Account disable is reversible
by the installation owner, including for a Brain owner needing recovery.

Every operation reloads effective access from PostgreSQL. Mutations serialize
against account disable and Brain access changes; an already authorized database
transaction may complete before revocation commits. New transactions after that
commit are denied. Workers revalidate at publication. Browser polling clears
inaccessible Brain content on 401/403/404 and refreshes permission state; hidden
buttons alone are not authorization. Profile permissions remain separate and are
implemented in the MCP slices.

### Optional single-organization OIDC

Environment inputs are `RECOLLECT_OIDC_ISSUER`, `RECOLLECT_OIDC_CLIENT_ID` and
`RECOLLECT_OIDC_CLIENT_SECRET`; all three or none must be supplied. Use an exact
single-tenant issuer over HTTPS, except loopback HTTP for a local test provider.
The callback is `{RECOLLECT_PUBLIC_ORIGIN}/api/auth/oidc/callback`. Metadata,
authorization/token endpoints and signing keys come from trusted discovery.
HTTP calls have a five-second timeout and do not follow redirects.
`RECOLLECT_OIDC_SCOPES` is a space-separated provider scope list, defaulting to
`profile groups`; `openid` is always included by the client. For Entra use `profile`
and configure group claims in the application registration; `groups` is not an
Entra OAuth scope. The issuer must name the actual tenant, not a multi-tenant alias.

`GET /api/auth/options` exposes configuration availability, not connected state.
`GET /api/auth/oidc/start` discovers the provider and redirects a browser through
authorization code flow with a confidential client, state, nonce and PKCE. Store
short-lived state/nonce/verifier server-side for five minutes and bind the callback
to a separate HttpOnly SameSite=Lax state cookie. No bearer tokens are logged.
`GET .../callback` consumes state once, exchanges code, validates ID-token signature,
issuer, audience, nonce and expiry with the maintained OpenID Connect library,
then establishes the same server-backed session. Errors redirect to a fixed local
sign-in error state; no provider error payload or open redirect is exposed.

`POST /api/team/oidc-accounts` explicitly provisions an account by exact configured
issuer and stable subject, plus a local display username. No email matching,
auto-linking or just-in-time signup occurs. Unknown subjects are denied.
Group claims must be a complete string array, at most 200 entries; missing,
malformed, `hasgroups` or `_claim_names.groups` overage indicators contribute
no inherited access and clear the prior snapshot. Direct grants and ownership
remain distinct. Entra group overage is denied rather than fetching arbitrary
claim-source URLs or silently widening access.

OIDC sessions and group snapshots expire at the earlier of ID-token expiry and
five minutes. Reauthentication obtains fresh signed membership (request login
with max_age=0); there is no stored refresh/access token. Provider unavailability
cannot extend stale membership. Internal account or mapping changes apply at the
next operation immediately; external group changes are bounded by five minutes
from the last successful authentication. Existing local users remain usable
during an OIDC outage. This is not a claim about arbitrary external credentials.

### UI and limits

An installation Team view manages invitations, account status, recovery and OIDC
subjects with explicit pending/empty/error states. Tokens are shown only on create
or reset. Brain Access shows independent permission sources, group mappings,
direct role controls and ownership transfer. The sign-in page supports invitation
acceptance and optional organization login. Bounded queries show at most 1,000
accounts/mappings/invitations; exceeding administration capacity returns 429.
The optional provider is not required by `./scripts/dev.sh`.

## Acceptance

Real HTTP handlers plus non-owner PostgreSQL prove invitation replay/expiry/reset,
local login and disabled-account denial, denied non-owner administration,
cross-Brain isolation, ownership/direct/group combinations, group expiry/overage,
removal and worker revocation with permitted controls. Exercise a real local OIDC
provider through discovery, signed code exchange and callback; distinguish this
from untested external Entra tenancy. Prove malformed callback/token denial and
outage behavior. Test actual browser enrollment and access controls. Run focused
tests, Rust/frontend checks and `./scripts/validate.sh` before closeout.

## Explicit Deferrals

Device credentials, profile grants, shared HTTPS packaging and full backup/recovery
drills belong to their named slices. OIDC is optional; no customer tenant or
external provider deployment is performed. A local provider test does not prove
an external tenant configuration.
