# Configure approved MCP connections and profiles

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

The desktop Brain catalogue configures approved tools and independent Use, Manage
and Share rights. It reads cached schemas without launching a provider, opening
its target or resolving credentials. Actual execution follows in the runtime
slice. See the [contract](../contracts/mcp-catalogue-and-profiles.md) and
[local proof](../mappings/mcp-catalogue-2026-09-22.md).

Start the local Compose service with `./scripts/stack.sh up` and open
`http://127.0.0.1:8787`. The operator needs the existing migration credentials
and owner configuration in the ignored `.env`; Brain connection administration
requires a browser session with Brain admin access. Profile Use is separate.

## Procedure

For an anonymous HTTP server, use **Connections → MCP servers → Add connection**.
Enter its name and URL, or select **Use Context7 · no API key**, then **Find tools**.
After inspecting the discovered tool names, choose **Add server**. This records
the approved manifest and a central connection in the selected Brain. Listing
tools does not execute them. Tool-group Use remains an independent permission.

Use **Use registered connector** for an existing definition, or **Import manifest**
for authenticated servers, local executables and private runners. The older
operator/import path below remains supported:

1. Prepare a non-secret definition JSON inside the repository. Its fields are
   `key`, `name`, `description`, `transport`, `command`, `arguments`, `placements`,
   `credential_aliases`, `configuration_schema` and `tools`. For stdio approve an
   absolute executable and literal argument vector; HTTP uses a null command and
   empty arguments. Each tool has `name`, `description`, `inputSchema`, optional
   `outputSchema` and `annotations`. Configuration/input schemas describe objects;
   configuration must set `additionalProperties: false`. Only local schema
   references are supported. Review the implementation and schemas before import.
2. The installation owner can register a new connector in **Connections → MCP
   servers → Add connection** when the catalogue is empty, or **Register another
   connector** otherwise. Choose the prepared JSON file (or paste it) and select
   **Register connector**. This uses the same validation as the CLI and never starts
   the backend. An identical repeat is idempotent; conflicting existing keys are
   rejected. Non-owner Brain admins see the operator prerequisite.

   Operators can also import or update a definition through the existing command:

   ```sh
   set -a
   source .env
   set +a
   target/debug/recollect-server mcp-definition-import .recollect/approved-connector.json
   ```

   Replace the example path with the prepared file. This atomically approves or
   updates its stable key, records metadata audit, and does not connect to it.
   An identical import is idempotent. The synthetic test manifest at
   `crates/server/tests/fixtures/mcp-catalogue.json` demonstrates the format but
   deliberately uses `/usr/bin/false`; it is not a usable connector.
3. Open a Brain's **Connections → MCP servers** tab. **Add connection** selects
   the approved connector, environment, fixed target, placement, runner reference,
   optional approved credential alias and schema-validated non-secret settings.
   Central placement has no runner reference. Local/private placement requires
   one; at this stage that reference is configured, not verified. Supply no secret
   values in JSON or target addresses.
4. Continue with **Configure profile & test**, or open **Tool groups → Create execution profile** and select up to 20 compatible connections.
   Brain-wide profiles contain Brain-wide connections only; environment profiles
   can include that environment and Brain-wide connections. **Inspect profile**
   edits memberships for managers and grants for sharers. Grants select an enrolled
   username or the configured OIDC issuer's group, with independent Use, Manage
   and Share checkboxes. Administrators and creators do not inherit Use. A sharer
   can explicitly select themselves and save a Use grant.
5. A user with Brain access and Use can **Inspect cached tools**, select the exact
   environment when required, inspect schemas and page through results. Listings
   identify both connection and tool name. **Configured · connection not checked** is the
   expected status; cached metadata does not prove availability or a live call.

## Verification

Check the Brain mutation audit for configuration/grant changes. Removing one
grant reports remaining effective rights from other direct/group/admin sources.
The browser refreshes every five seconds and clears inaccessible tools or stale
editors after observing a change. A delegated manager can edit a profile without
Use or Share, and a sharer can grant Use without connection administration.

Discovery returns at most 20 descriptors per page. Limits are 100 installation
definitions, 100 connections/profiles per Brain and 100 grants per profile.
Paired-device discovery requires an owned immutable context/tool operation and
matching environment; browser configuration is not exposed to devices.

## Failure and Recovery

- Empty catalogue: approve a definition before creating a connection. No external
  provider or credential is contacted by inspection.
- Invalid settings: correct them against the approved schema. Known credential
  keys/markers are rejected; this is not a universal secret-string detector.
- Stale edit (409): reopen the record before applying changes. A name collision or
  incompatible environment membership also reports a conflict.
- Changed definition: existing configurations are revalidated. Fix invalid settings
  or disable the connection while preserving those settings. To withdraw a
  definition, use `target/debug/recollect-server mcp-definition-disable KEY` with
  the same operator environment. Reimport approved content to enable it again.
- Environment deletion conflict: reassign referencing connections/profiles first;
  disabling preserves their environment reference.
- Access/refresh failure: cached detail is hidden; reload after restoring access.
  A new discovery rechecks current Brain, account/device and profile rights.
- Capacity reached: the service reports an explicit capacity error. No automatic
  purge of configuration identities or silent truncation is performed.

Configuration, credentials, runtime calls, Vault delivery and private-runner health
are separate states. This catalogue provides no Run action or connectivity claim.
