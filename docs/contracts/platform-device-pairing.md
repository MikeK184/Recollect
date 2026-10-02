# Companion pairing and device authority

Status: accepted

## Source

[Runtime ADR](../adr/0003-product-runtime.md), [team access](platform-team-access.md),
[durable work](platform-durable-work.md) and the
[platform epic](../roadmap/epics/product-platform.md). The accepted foundation
requires browser-approved companions with individually revocable credentials in
the OS credential store. Routine details are authorized by the full-product goal.

## Contract

### Pairing lifecycle

`POST /api/devices/pairings` accepts a device name (1–120 trimmed characters), an
optional `host_kind` (`codex`, `claude_code` or `opencode`) and an optional
`integration` marker (`mcp` or `plugin`; default `mcp`), and creates a five-minute
request. Unknown marker values are rejected with 400. Return a private UUID
`device_code`, an eight-hex-character public `user_code`, verification URL, expiry
and two-second poll interval.
The companion prints only the URL and public code. There are at most 1,000 active
requests installation-wide; exhausted admission returns 429. Expired requests are
cleaned up without affecting active device credentials or audit history.

A signed-in browser opens `/devices?code={code}`. `GET /api/devices/pairings/{code}`
shows the requested name/code/expiry and pending disposition. It does not reveal
the private code or credential. `POST /api/devices/pairings/{code}/approve` with a boolean decision
requires a current browser session and CSRF. Approval binds the device to that
account, persists the pairing's `host_kind` and `integration` on the device record
(a reused same-name device is updated in place), records audit and creates a random
bearer credential valid for 30 days.
Decline is terminal. A second approval is 409; expiry/missing code is 410. The
screen asks the person to compare the code with their own companion request.
No device can approve another pairing or administer accounts through its token.

`POST /api/devices/pairings/poll` accepts the private device code. Pending returns
state only; declined/expired returns 410; approved returns the same device identity
and bearer token until acknowledged. This handles a lost poll response without
issuing multiple credentials. Polls faster than one second return 429. The issued
credential cannot authenticate before acknowledgement.

The companion writes the credential to its OS store, then calls
`POST /api/devices/pairings/finish` with the private code. This atomically marks the
device claimed. Repeating the same acknowledgement succeeds while the request
exists. If the response is lost, the companion can verify the saved credential
through a normal authenticated call. Expired unacknowledged credentials remain
unusable. `POST /api/devices/pairings/cancel` cancels pending/unclaimed requests;
it cannot revoke a device already acknowledged. Approval after account disable
or with an expired session is denied.

### Authenticated operations and revocation

Device calls use `Authorization: Bearer {token}`, where token is an opaque UUID. A supplied invalid bearer
never falls back to a browser cookie. Verify the device is claimed, unrevoked,
unexpired and owned by the enabled principal on each call. OIDC principals also
need a fresh five-minute identity snapshot; browser reauthentication refreshes
that shared identity snapshot without restoring any revoked device. Knowledge
roles remain ownership/direct/fresh-group grants from the team contract.

Bearer calls do not require browser CSRF. Browser session, local account recovery,
team administration and pairing approval remain browser-only. Brain operations
use the same principal/grant checks for either identity transport. A device does
not imply a Brain binding, workspace scope or any MCP profile-use permission.

`GET /api/devices` lists only the actor's safe device metadata. The browser can
revoke one of its devices with `DELETE /api/devices/{id}`. A bearer can revoke only
itself. Foreign IDs return 404; repeat revocation is harmless and does not reissue
credentials. At most 20 unrevoked, unexpired devices per account can be enrolled.
`POST /api/devices/revoke-self` accepts possession of the device token even after
account/identity/device expiry, solely to revoke that token's record. It returns
204 for an unknown or already revoked token and grants no read or renewal power.
This allows reliable companion unpair without confusing an expired OIDC identity
with confirmed server-side device revocation.
Expiry/revocation and account disable deny new managed calls. An authorized short
database transaction may finish before a concurrent revocation commits.

The common transaction boundary binds and locks the device when present. Durable
jobs retain the submitting device ID, and workers revalidate it at publication.
Revocation of one device cannot cancel another's permitted publication. Tokens,
private polling codes and driver payloads are excluded from audit/list responses
and logs. Mutation audit records the principal separately from device identity.

### Companion and UI

The plugin's `connect` command prints the verification link and waits up to five
minutes while the user approves it in the browser, sending its own host kind and
the `plugin` integration marker. `whoami` proves a real authenticated call;
`brains` lists actual permitted Brains; unpairing revokes this device before
deleting its stored credential. An already-revoked/expired token can be removed
locally. A network failure preserves the credential for retry. Forgetting a local
credential explicitly removes only the local copy and tells the user to revoke the
server record in the Brain's Agents list or the Devices route. Pairing does not
overwrite an existing local profile.

`RECOLLECT_URL` selects an HTTPS service (loopback HTTP is allowed locally).
The default is `http://127.0.0.1:8787`. Reject URLs containing user-info, query or
fragment. `RECOLLECT_DEVICE_PROFILE` defaults to `default` and selects a local
OS-store entry for that endpoint. Store endpoint, device ID and token together
using the native platform credential store; no source/config credential file or
password hash is introduced. Missing/locked/unavailable storage fails explicitly
without falling back to a pretend or unencrypted credential store. The companion
continues to depend only on protocol/HTTP/OS-store libraries, never server database
drivers. Provider tokens and browser session cookies are not copied to it.

The Devices route `/devices` is available to every signed-in account by direct URL
for pairing approval (including across a sign-in redirect) and the complete account
device list; it is no longer listed in global navigation, and per-Brain agent
visibility lives on the Brain's Agents surface. It shows name, creation, last-use,
expiry and claimed/revoked status, with pending/error/empty states and revocation
controls.
Companion messages distinguish waiting, denial, expiry, connection failure and
credential-store failure; no secret is printed in error output.

## Acceptance

Real handlers/database tests cover approval/decline, one-use state, retryable
delivery/acknowledgement, expiry/capacity, missing/invalid bearer, foreign-device
denial, account/group changes, per-device revocation and queued publication with
an independently permitted device as control. Exercise the actual native companion
and browser through pairing, OS-store persistence across processes, authenticated
reads and unpair/revocation. Isolate temporary test entries under unique Recollect
profiles and remove only those created by the test. Preserve unrelated credentials
and platform configuration. Run focused tests, Rust/frontend checks and
`./scripts/validate.sh` before archive.

## Explicit Deferrals

Workspace bindings, repository capture, stdio MCP and private runner roles belong
to their named slices. No pairing enables arbitrary shell execution. Local macOS
native-store proof does not establish runtime proof on other operating systems.
