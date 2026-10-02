# Connect and manage a Recollect device

> Plugin migration: [ADR 0018](../adr/0018-plugin-managed-agent-memory.md)
> makes the packaged plugin the normal install/connect path for Codex, Claude Code
> and OpenCode. [Package instructions](../../plugins/recollect/README.md) describe
> the delivered implementation. Native ordinary-launch acceptance passed; the
> legacy procedures remain for draining existing queues and advanced use.
> Do not remove an old setup or revoke its device before its pending captures have
> drained under their original endpoint, Brain, task and binding. Do not enable both
> capture paths for one session. No credentials or host trust settings are migrated
> silently.

## Connect the packaged plugin

Use **Agents → Connect coding agent** and the [package instructions](../../plugins/recollect/README.md).
The bundled `recollect-plugin connect --url URL --brain UUID` starts the existing
browser device-approval flow, saves the approved credential in the OS store and
verifies access to the Brain. Normal coding sessions then load that credential
automatically. For an existing access token, `--token-stdin` accepts it through
standard input; never put the value in arguments or host configuration.

macOS SecurityAgent is the operating system's credential-consent dialog. A prompt
for `recollect-plugin` asks whether that executable may read the stored Recollect
credential. It is not a Recollect reasoning agent, a memory-quality test or a
request for an LLM password. Approve only the intended executable and profile.
Replaced binaries can require renewed OS consent. Linux uses an unlocked Secret
Service session; there is no plaintext credential fallback.

`recollect-plugin status` checks the selected connection. `disconnect` disables
the owned runner, revokes the current device and removes its OS-store credential.
Drain intended pending captures before disconnecting; revocation blocks their
publication. Switching Brain/device through `connect` preserves original queue
references. Each Brain's **Agents** list shows its agents with per-Brain last use
and supports revocation (account-wide); the **Devices** route, reachable by
direct URL, lists the complete account device set. Pairing never creates
execution grants or changes Brain membership.

Ordinary memory starts no execution runner. `connect --with-runner` enables one;
reconnect without that flag to disable it. A private registration additionally
uses `--runner-id UUID`. See [scoped tools](agent-memory-tools.md).

## Legacy native profile compatibility

The commands below operate the earlier standalone companion profile. Keep that
credential while draining any original capture queue; plugin setup does not
silently copy or revoke it. Previous companion proof does not establish the new
packaged install: use the [plugin evidence mapping](../mappings/plugin-session-memory-2026-10-01.md)
for current acceptance status.

### Legacy prerequisites

Use the delivered companion with a running Recollect installation and a signed-in
account. macOS native-store integration was verified locally. Linux Secret Service also has isolated native-host fixture coverage; other OS
stores are not implied by those tests.

### Legacy procedure

From the repository root, with `./scripts/dev.sh` running:

```sh
cargo run -p recollect-agent -- pair "Development laptop"
```

Open the printed link, sign in if needed, compare the public code with the
companion, and choose **Approve device**. The companion writes its credential
to the OS store, acknowledges pairing and verifies an authenticated call.
No token is printed. On macOS the stored item names the companion, bridge and
runner as trusted applications. Approve any Keychain prompt only for the intended
executable and profile; use Always Allow when you want that trusted binary to load
the credential on later launches. Rebuilding or replacing a binary can change its
identity. A fresh pairing refreshes the trusted list; old items are not silently
rewritten. Verify access from fresh processes rather than treating the saved list
or browser pairing approval as connection proof:

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
more. Re-pairing with the same device name (case-insensitive, ignoring
surrounding whitespace) reuses that device record and rotates its credential
instead of adding a row; the expiry resets to 30 days. A claimed device expires after 30 days and must be paired again.

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
