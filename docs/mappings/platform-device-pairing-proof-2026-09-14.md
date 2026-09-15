# Companion pairing dependency and local proof

Observed: 2026-09-14
Confidence: verified

## Sources and Method

The [device contract](../contracts/platform-device-pairing.md) governs behavior.
Context7 resolved `/open-source-cooperative/keyring-rs`. Its current response
described the 4.x API, while the first official page checked described 3.6.3;
the implementation then verified and used the current 4.2.0 interface:

- [Keyring 4.2.0 native OS store](https://docs.rs/keyring/4.2.0/keyring/)
- [Keyring Entry API](https://docs.rs/keyring/4.2.0/keyring/struct.Entry.html)
- [Maintained upstream source](https://github.com/open-source-cooperative/keyring-rs)

Resolved `keyring = "4"`, with the macOS native adapter using Security Framework.
Inspected the installed library's store selection and error paths. No custom
keychain or personal search-list change was introduced. Errors are mapped to
safe messages because native error payloads can contain secret bytes.

Used actual handlers/PostgreSQL, the isolated Dex provider, native companion
processes and isolated Playwright Chrome. Generated API IDs were checked before
TypeScript generation. Screenshots of Devices and approval on desktop/mobile were
inspected. The existing in-app browser plugin remained unavailable as recorded
in bootstrap evidence; Chrome testing used the repository-owned test harness.

## Observations

| Behavior | Evidence |
| --- | --- |
| Pairing | Approval binds the actual browser account. Repeated approval fails; private polling repeats the same credential until acknowledged; repeated acknowledgement succeeds. Pre-acknowledgement authentication is denied. |
| Errors | Declined/expired requests, fast polling and global/per-account admission limits are exercised. Invalid bearer does not fall back to a valid cookie. Device administration remains browser-only. |
| Isolation | Foreign device IDs and unrelated Brains are denied. Device lists omit tokens; mutation audit records device and principal separately. |
| Publication | A queued job submitted by a revoked device cannot publish; another device remains authorized. Browser retry adopts current browser provenance. Disabled accounts can still revoke their own token without read access. |
| OIDC | Real signed sign-in supplies group access. Advancing the stored membership deadline denies device authentication; real reauthentication restores an unrevoked device but never a revoked one. |
| Native OS store | Browser approval followed by separate `whoami` and `brains` processes succeeded. Duplicate profile was denied. Browser revocation denied the next call; unpair removed the native entry. Pairing again and unpairing an active device also succeeded. Only the test's unique entry was removed. |
| UI | Four complete browser workflows passed, including companion pairing, team access and organization login returning to the pending pairing. Desktop/mobile screens have no horizontal overflow in the tested viewport. |
| Recovery | Repeated polling/acknowledgement and cancellation after acknowledgement preserve a claimed device. Connection/store failures are explicit and unpair preserves credentials on network failure. |

Six core Rust scenarios and one dedicated real OIDC scenario passed. The focused
native/browser test passed in 10.4 seconds; the final four-workflow browser run
passed in 24.2 seconds and a fresh worker drained its persisted jobs. Clippy with
warnings denied, rustfmt and frontend build passed. The API has 40 unique operations.
Governance initially rejected angle-bracket token notation in the contract;
the token notation and abbreviated paths were clarified before final validation.
Governance lint and all 32 checker tests then passed. The normal local startup
migrated existing data; real owner login and `/api/devices` succeeded, and both
HTTP readiness and native companion health reported ready.

## Translation and Limits

OS-store proof is local macOS evidence, not Windows/Linux runtime proof. No real
external identity tenant or shared deployment was configured. OIDC expiry is
tested by advancing persisted deadlines. No source hashes, password hashes or
strict product versions were added. Device pairing does not deliver workspace,
capture or MCP execution functionality.

## Follow-up

Continue with evidence collections and workspace scope. Device identities now
provide the authenticated companion transport for those slices.
