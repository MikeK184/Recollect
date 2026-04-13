# Local team access

## Purpose and Prerequisites

Use the delivered [team contract](../contracts/platform-team-access.md) with
`./scripts/dev.sh`. The installation owner manages accounts; Brain admins manage
access within their Brains. Ownership, direct roles and organization groups are
independent sources of permission.

## Procedure

1. Sign in as owner and open **Team → Add teammate → Local invitation**.
   Choose a username and share the displayed link privately. Recollect sends no
   email. The link expires after 24 hours and is accepted once.
2. The teammate opens the link and chooses a password. Share a Brain separately
   through **Manage access**, entering the enrolled username and direct role.
3. Use the same form to change a direct role. Removing a grant reports remaining
   effective access. Ownership or a group mapping may still authorize the member.
4. The current Brain owner can transfer it to an enabled member in that panel.
   Existing contributor/audit identity does not change. The previous owner has
   only any remaining independent grants after transfer.
5. **Team → Disable** revokes that account's sessions and prevents subsequent
   operations and worker publication. Re-enable requires a new sign-in.
   **Reset sign-in** revokes local credentials/sessions and issues a new invitation.
   The environment-backed installation owner uses the CLI recovery in the
   [local development runbook](local-development.md).

Local teammate passwords are stored without hashing in the ignored 0600
`.data/credentials.json`, following the user's development instruction. PostgreSQL
stores random credential references. Preserve that file with the database and
the ignored `.env`; a database-only restore cannot restore local sign-in secrets.
`RECOLLECT_CREDENTIAL_FILE` can select another repository-local ignored path.
This file supports one local API writer; shared packaging is an operations slice.

## Optional organization sign-in

Configure the ignored `.env` with `RECOLLECT_OIDC_ISSUER`,
`RECOLLECT_OIDC_CLIENT_ID` and `RECOLLECT_OIDC_CLIENT_SECRET`, then restart the API.
Register the exact callback `{RECOLLECT_PUBLIC_ORIGIN}/api/auth/oidc/callback` with
the provider. Use a single-tenant HTTPS issuer; loopback HTTP is permitted for
the isolated integration fixture. Do not paste credentials into documentation.

`RECOLLECT_OIDC_SCOPES` defaults to `profile groups`. Entra registrations should
use `profile`, enable the required group claim and set their tenant-specific
issuer. Add an **Organization identity** in Team using the exact provider subject
and a local username. Email does not link or enroll an account. Add exact group
IDs/names and roles through the Brain's Access panel.

The sign-in button reports configured availability. Successful discovery, signed
code exchange and a session are required to establish connectivity. OIDC accounts
must reauthenticate after at most five minutes; group grants expire at that bound
even during provider failure. Internal mapping/account changes apply immediately.
Missing/incomplete/overage group claims contribute no inherited access. Direct
grants and ownership remain independent, visible sources of access. No Microsoft
Graph group-overage fetch or stored refresh token is used.

## Verification and Recovery

```sh
./scripts/test-platform.sh
./scripts/test-oidc.sh
./scripts/test-oidc.sh ui
./scripts/validate.sh
```

The OIDC checks create an isolated Dex 2.45.1 container on loopback port 5556 with
its synthetic mock identity and a random runtime client secret. They use temporary
test databases and remove their own provider afterward. This provider is not
enabled by normal startup and must not be configured as a real account provider.
The checks prove discovery/JWKS, signed code exchange, callback binding/replay,
membership expiry, group access and browser sign-in. No external Entra tenant is
configured or verified by those checks.

- Lost invitation response: use **Reset sign-in** for the reserved local account
  to issue a new link. A revoked/expired/used link returns a clear error.
- Missing/corrupt credential file: owner login still works; restore the ignored
  file from its local backup or recover affected members through new invitations
  after repairing the file. Do not replace a valid file as a restart procedure.
- Provider unavailable or identity denied: local owner/team login remains
  independent. Verify the issuer, client registration, callback, exact subject
  and group configuration. Error pages do not expose provider token payloads.
- Apparent access after grant removal: inspect ownership, direct grants and fresh
  group mappings separately. Disable the account if all installation access must
  be revoked. Already authorized short database transactions may finish before a
  concurrent revocation commits; later transactions and worker publication recheck.
