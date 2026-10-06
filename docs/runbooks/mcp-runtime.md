# Execute approved MCP tools

For the current route/menu map, see the [desktop guide](desktop-experience.md).

## Purpose and Prerequisites

The [runtime contract](../contracts/mcp-runtime-and-credentials.md) governs managed
central/local execution, credentials, leases and uncertain completion. Use a
migrated local stack, an operator-approved connector, a Brain connection and a
profile with an explicit **Use** grant. Configuration, cached discovery, a healthy
runner and a successful tool response are separate observations. Brain admin,
Manage and Share do not imply Use.

The server owns the central executor. A paired companion must be running explicitly
for local placement. See [optional Vault and private runners](mcp-vault-and-private-runners.md)
for registered private hosts and per-connection Vault delivery. No customer
connector is installed by the test harnesses or this procedure.

## Procedure

Start the local stack with `./scripts/dev.sh` and open
<http://127.0.0.1:8787>. In a Brain, open **Connections → Profiles**, inspect an execution profile and open **Inspect
cached tools**, select **Run tool**, and supply non-secret object arguments matching
its schema. The selected environment/profile stays fixed. The desktop call
inspector shows queue/start/run state, selected runner, outcome, eligible retained
output and reconciliation. **End tool session** drains that page's connections;
already active calls can finish. Mobile layouts are outside the current scope.

For a companion, pair an endpoint/profile using the
[device procedure](device-pairing.md). Set a local connection's runner reference
to `device:` followed by that paired device's UUID. Start its executor:

```sh
cargo build -p recollect-agent
target/debug/recollect-agent mcp-runner .data/mcp-receipts/device
```

The build produces `recollect-agent` and `recollect-mcp-runner`. Install them
side-by-side; the public command executes the helper and preserves its terminal
and shutdown signals. Ordinary capture hooks retain their small startup path.

The directory is created privately and bound to that endpoint/device. An existing
directory must have owner-only permissions. Keep it for restart recovery; do not
reuse another device's directory. Ctrl-C stops admission and drains active calls.
The runner uses outbound authenticated HTTP only and never receives DB credentials.

Use `RECOLLECT_URL` and `RECOLLECT_DEVICE_PROFILE` for the same paired identity.
Create/select a task and begin its immutable `tool` operation using the
[scope commands](workspace-scope.md). Put these fields in a JSON call file:

- `request_id`: a fresh UUID for a deliberate operation; preserve it for retries.
- `profile_id`, `connection_id`, `tool_name` and non-secret object `arguments`.
- `operation_id`: this device's owned `tool` operation; `environment_id` must agree
  with its recorded scope.
- `client_session_id`: a stable UUID for this client's compatible connection reuse.
- `timeout_seconds`: 1–3,600; omitted means 300.

```sh
target/debug/recollect-agent mcp call BRAIN_UUID request.json
target/debug/recollect-agent mcp status BRAIN_UUID CALL_UUID
target/debug/recollect-agent mcp status BRAIN_UUID
target/debug/recollect-agent mcp cancel BRAIN_UUID CALL_UUID
target/debug/recollect-agent mcp release BRAIN_UUID SESSION_UUID
```

Use `-` instead of the JSON filename to read stdin. No CLI input accepts a command,
target, runner or credential override. Browser calls may explicitly use Brain-wide
or selected environment scope without a companion task; device calls require one.

Credential bindings use `RECOLLECT_MCP_CREDENTIALS_FILE` on the selected executor.
Central execution falls back to the private sibling `mcp-bindings` file when
this override is absent. Browser-managed development credentials are explained
below; paired runners keep their separately configured providers.
The file holds references, not values. Example shape (replace placeholder UUIDs):

```json
{
  "bindings": [{
    "connection_id": "CONNECTION_UUID",
    "alias": "approved-read-alias",
    "runner_reference": "central",
    "environment": {
      "CONNECTOR_API_KEY": {"provider": "environment", "name": "MY_CONNECTOR_KEY"}
    },
    "headers": {}
  }]
}
```

An OS-store reference is `{"provider":"os_store","account":"entry-name"}` in
service `recollect-mcp`. HTTP bindings use header keys with
`{"source": {"provider":"environment","name":"MY_CONNECTOR_KEY"},"prefix":"Bearer "}`.
Reserved loader, routing and Recollect/device variables cannot be destinations.
Values must contain 4–16,384 bytes. Missing/locked entries fail before dispatch.
OS-store/reference-file changes are read on the next operation; `.env` changes
require restarting the executor. Resolved credential changes drain old instances
while their active calls may finish. Provider stderr and raw SDK diagnostics are
suppressed; retained results pass shared and exact-value redaction.

## Browser-managed connector setup

The installation owner manages reusable definitions at **Connectors**, outside
individual Brains. Add a URL or paste/import JSON, TOML or YAML. Parsing only
reads the configuration. Command imports remain inert and must match an approved
command and arguments before a Brain connection can use them. Explicit HTTP
inspection loads bounded metadata; it neither calls tools nor grants Use rights.

Under **Brain → Connections**, an owner who is also a Brain administrator may
enter a masked Bearer/header or stdio environment value for an approved alias on
a central connection. Save first validates the current connection revision and
its approved transport. The installation development provider atomically writes
values to `credentials.mcp-secrets.json` and references to
`credentials.mcp-bindings.json`, beside the configured `credentials.json` login
file. Other credential-file stems produce matching sibling names. Values do not
enter connection settings, database audit, returned metadata or browser storage.
The value file must remain owned by the executor and mode 0600; missing, unsafe
or inaccessible files fail closed before tool dispatch. Rotation preserves the
binding's opaque key and changes the resolved credential generation.

An existing `RECOLLECT_MCP_CREDENTIALS_FILE` override remains authoritative;
owner provisioning updates only the exact central connection/alias binding and
preserves other bindings and Vault sources. Vault and paired/private execution
still use their existing provider configuration. This UI explicitly identifies
the installation-local development file; it does not promise an OS store or a
production Vault. A failed credential save may leave a configured connection:
retry the masked credential step or reopen that connection; no tool is run.

## Verification

### Automatic observation capture

In the Brain's **Session capture → Capture policy**, enable automatic capture
and **Capture managed tool results**. This permits future eligible writer/admin
results to become evidence readable by that Brain's knowledge readers. It does
not grant profile Use or enable model transmission. Reader executions continue
without publishing knowledge, and enabling capture does not import older calls.

After a managed call completes, **Captured observations** in its inspector shows
the publication state. **Inspect captured evidence** opens the retained source;
the Brain's published activity shows the same original scope and call identity.
The source records the reported outcome, original target and receipt lineage.
Native `mcp status` and MCP call-history responses include observation records.
Capture makes no model request. Automatic learning follows the separate standing
model policy, including permission to transmit tool output.

Unknown completion and later receipts remain separate observations. A filtered
event can have no retained text; exclusions apply to the whole input/output before
truncation. Retry state concerns evidence publication and cannot repeat the tool.
Pending text has a 30-day default deadline, a 1,000-record/64-MiB Brain queue limit
and retries up to 60 seconds apart. Counts describe visible calls. A capacity gap
does not discard earlier pending evidence or turn a successful tool into failure.

Source **Erase** includes this call's other published observations and fences
queued or late copies. Native runners check pending receipt IDs before resend;
disconnected runners learn deletions at their next successful check-in, with the
existing one-hour receipt limit still applying. Erasure does not undo an external
tool's side effect. Explicit retention-policy changes apply to published evidence
from its original capture time; they cannot restore removed bytes or prolong the
pending outbox's original deadline.

### Execution and receipt checks

**Runner available** means its last bounded lease remains active. **Connected**
means the executor reported an initialized SDK connection. **Succeeded** requires
an actual complete connector response, which does not itself verify every claimed
external effect. Caller history never bulk-returns raw results. Result reads
require current Brain and profile Use authority and expire after one hour.

The [dated evidence](../mappings/mcp-runtime-2026-09-22.md) records real central and
paired native stdio/Streamable HTTP calls, process cleanup, credential isolation,
recovery and desktop behavior. Synthetic executables and definitions are installed
only in owned disposable test databases. `./scripts/test-mcp-runtime.sh` exercises
the shared SDK boundaries. The API/runtime integration cases are under
`crates/server/tests/platform/mcp/`; build `recollect-agent` and the `mcp-fixture`
example before running their ignored tests with the repository database variables.

## Failure and Recovery

- **Queued / runner unavailable:** start the selected runner, or cancel. There is
  no central fallback for local calls. Queues expire after ten minutes. Capacity
  deferral only affects attempts that have not dispatched.
- **Already active runner:** wait for its 30-second lease; never force a second
  healthy owner. Loss of authority cancels/drains owned work and fences new sends.
- **Unknown completion:** never blindly repeat the effect. An approved receipt
  policy enables **Look up connector receipt** as a separate read. It preserves
  the original unknown attempt and records the connector-reported observation.
- **Evidence resolution:** select accessible retained evidence in the inspector,
  or use `mcp resolve BRAIN_UUID CALL_UUID resolution.json` with `request_id`,
  `outcome`, `source_version_id` and `explanation`. Agents can use the same API;
  there is no human review queue. Explanations follow source erasure/retention and
  are kept at most one hour. Minimal attribution remains.
- **Native receipt lookup:** `mcp reconcile BRAIN_UUID CALL_UUID lookup.json`, with
  fresh `request_id` and `client_session_id`. Changed original configuration blocks
  lookup instead of querying a replacement target.
- **Lost completion upload:** keep the private SQLite outbox. Recovery retries
  the sanitized receipt only. At most 64 receipts live for one hour; saturation
  or storage failure pauses admission. Fenced receipts remain quarantined until
  expiry unless an observation erasure removes them sooner. If the removal check
  is unavailable, retained receipts wait and new admission pauses. The central
  default is `.mcp-receipts/central` under the artifact directory;
  `RECOLLECT_MCP_OUTBOX_DIR` can select another private central directory.
- **Forced exit after dispatch:** recovery reports unknown, and a later valid
  receipt can reconcile it. The supervisor closes only its owned process group.
  Hosted HTTP services remain running; no persisted PID is used for recovery kills.
- **Expired inputs/results:** GET retains minimal disposition. Replaying an expired
  request returns `mcp_replay_expired`; do not invent another request ID merely to
  retry an effect whose completion is uncertain.
