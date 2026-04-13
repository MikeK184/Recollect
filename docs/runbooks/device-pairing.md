# Pair and manage a native companion

## Purpose and Prerequisites

Use the delivered companion with a running Recollect installation and a signed-in
account. macOS native-store integration was verified locally. Other OS stores
are supplied by the keyring library but have not been exercised here.

## Procedure

From the repository root, with `./scripts/dev.sh` running:

```sh
cargo run -p recollect-agent -- pair "Development laptop"
```

Open the printed link, sign in if needed, compare the public code with the
companion, and choose **Approve device**. The companion writes its credential
to the OS store, acknowledges pairing and verifies an authenticated call.
No token is printed. Inspect the result from fresh processes:

```sh
cargo run -p recollect-agent -- whoami
cargo run -p recollect-agent -- brains
```

The default endpoint is `http://127.0.0.1:8787`. Set `RECOLLECT_URL` for another
endpoint; HTTP is allowed only on loopback. `RECOLLECT_DEVICE_PROFILE` selects
an independent local profile, default `default`. Entries are endpoint-specific.
An existing profile is preserved until explicitly unpaired or forgotten.
The device acts as its account, using current Brain permissions. Pairing grants
no workspace binding or execution-profile permission.

Open **Devices** to see your devices, last use, expiry and status. Revoke a
selected device there, or revoke and delete the local entry with:

```sh
cargo run -p recollect-agent -- unpair
```

## Failure and Recovery

Pairing expires after five minutes. Decline/cancel requires a new request. Up to
20 live devices may belong to an account; revoke unused devices before adding
more. A claimed device expires after 30 days and must be paired again.

Locked/unavailable OS storage fails explicitly. Unlock the store and retry;
there is no credential-file fallback. Connection errors preserve an existing
credential. If acknowledgement is uncertain after storage, try `whoami`; use
`unpair` before repeating pairing when it cannot be recovered. `unpair` can
confirm revocation even when account/OIDC/device validity has expired.

If the service is permanently unavailable, `cargo run -p recollect-agent -- forget`
removes only this endpoint/profile entry. It does not revoke the server record;
revoke that record in Devices when the service is available.

An OIDC device needs a fresh identity snapshot. Sign in through the browser at
least every five minutes while using it. Refreshing identity cannot restore a
revoked device. Account disable and current Brain grants also apply on every
managed call and before queued work publishes.

## Verification and Limits

`./scripts/test-ui.sh --grep 'browser approves'` starts disposable product data,
an isolated Chrome session and real native companion processes. It creates and
removes a unique Recollect OS-store entry. Browser approval, fresh-process reads,
duplicate-profile rejection, browser revocation and active/revoked unpair pass.
Screenshots are under `.cache/ui/`. No unrelated credentials are enumerated.

`./scripts/test-platform.sh` covers device lifecycle, lost-response replay,
expiry/admission, isolation and worker revocation. `./scripts/test-oidc.sh` also
checks device identity expiry and reauthentication with a real local provider.
This is local delivery; shared deployment and workspace/MCP roles remain separate.
