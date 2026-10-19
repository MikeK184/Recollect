# Exercise cross-machine agent coordination

## Purpose and Prerequisites

Verified: 2026-10-18. The [dated proof](../mappings/agent-coordination-proof-2026-10-18.md)
records all 15 checks passing on macOS and Ubuntu 24.04, zero canonical model
requests and owned-resource cleanup. The
[archived pack](../roadmap/execution/archive/mcp-agent-coordination.md) records
the delivered scope and host/deployment limits.

Use a macOS checkout with Rust/Cargo, Python 3.11 or newer and the repository Docker wrapper
available. The fixture creates its own PostgreSQL/pgvector database and Neo4j/GDS
service for normal server bootstrap and analytical ownership checks, runs the
newly built Rust server natively, and runs Paul's peer plus the real native plugin
in Ubuntu 24.04. The Linux compiler image is Debian-based; the tested peer runtime
is Ubuntu. Its isolated Secret Service stores synthetic device credentials.

The fixture uses made-up `team-fixture` and `private-fixture` Brains, Mike/Paul
accounts, repository origins and file intentions. It loads no normal `.env`,
uses no existing Brain, and configures no model provider. The contract is
[Brain-scoped coordination](../contracts/mcp-agent-coordination.md); the
[A2A research](../mappings/agent-coordination-a2a-research-2026-10-18.md) explains
the narrow adapter and host limitations.

## Procedure

From the Recollect checkout:

```bash
cargo build --locked -p recollect-server --bin recollect-server
./scripts/build-agent-coordination-peer.sh
python3 scripts/test-agent-coordination.py
```

The Linux build writes only repository-owned caches under
`.cache/agent-coordination-linux/`. Each test run records resources in a unique
`.cache/agent-coordination/proof-*/resources.json`. Test output identifies checks
as they actually pass. Inspect that run's booleans and final process exit code;
the existence of a run directory alone proves nothing.

The fixture starts a fresh database/container with an ownership label, creates
accounts/devices through product paths, registers equivalent repository origins,
and exercises overlap, inbox, negotiation, A2A and MCP. Native plugin proof uses
the regular connection/OS-store path, documented hook JSON and the CLI. Its
synthetic hook receipt is `native-hook.json`; readable watch output is
`native-watch.txt`. Host hook invocation is distinct from an installed coding
host/model consuming that output.

Normal completion and assertion failures both attempt cleanup of the exact
labelled containers and the owned native server. For an interrupted run, use its
exact retained directory:

```bash
python3 scripts/test-agent-coordination.py --fixture .cache/agent-coordination/proof-RUN_ID --cleanup
```

Replace `RUN_ID` with the actual existing run identifier. The cleanup command
checks ownership before removing those fixture containers. Do not substitute a
production container or broad Docker pruning command. Private environment/log
files are retained in that mode-0700 directory for diagnosis; do not print or
publish them. `resources.json` and the two synthetic native receipts are the
intended shareable proof artifacts after inspection.

## Verification

Require the full test process to exit successfully and all 15 expected check flags
to be true. Verify the resources record identifies Ubuntu 24.04, the native
macOS server, fabricated Brain, binary/fixture hashes and cleanup completion.
The final verified run is `.cache/agent-coordination/proof-c00339f9482e/`, completed
at `2026-10-17T23:12:22Z`. `model_calls: 0` requires a zero-row assertion against the fixture canonical
model-request ledger. It establishes no recorded provider request in this
isolated run, not answer quality or a measurement of production traffic.

The proof must distinguish:

- A configured credential from a current authenticated session lease.
- Server acceptance from durable pull, streamed delivery and native hook output.
- An advisory proposal/acceptance from a file lock or authorization to execute.
- Native hook output from actual model consumption in an installed host.
- A successful Ubuntu-container peer from a second physical Mac or deployment.

The A2A card advertises the synchronous message-only HTTP+JSON 1.0 skill. Native
coordination SSE supplies push delivery separately; do not claim A2A task streams
or webhook support. Reconnect can redeliver event IDs. Pull/ack remains available
for retained messages after presence expires, subject to current access.

## Failure and Recovery

A failed build is not a skipped test. Repair the reported source/dependency issue
and rebuild the changed native binary before rerunning. A missing Linux binary
fails the fixture rather than omitting native acceptance. Use a fresh run after
partial data mutations; retained `--prepare`/`--fixture` support is for controlled
debugging, not a way to reinterpret partial checks as complete.

Connection unavailable: show degraded state and retain the durable cursor.
Known access denial/deletion suppresses cached automatic delivery. Canonical
Brain deletion erases pending goals, intentions and inbox payloads for the exact
binding, with a durable fence against late writes/new sessions. Busy local cleanup
is retryable; it is not a successful erasure. A peer with no
recent hook or independently verified host liveness stops renewing; long silent
tools can appear expired. Foreground watch remains active until stopped. A
90-second lease or a cached event is not proof that a coding host is working now.

No production endpoint, installed host configuration, normal keychain item,
unrelated container/volume, or customer repository is a fixture cleanup target.
